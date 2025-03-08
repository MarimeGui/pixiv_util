mod individual;
mod series;
mod single;
mod user_bookmarks;
mod user_posts;

use std::{
    env::current_dir,
    fs::read_dir,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Result};
use tokio::{
    fs::create_dir,
    spawn,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot::{self, Sender},
    },
    task::JoinSet,
};

use crate::{
    gen_http_client::SemaphoredClient,
    incremental::{is_illust_in_files, list_all_files},
    update_file::create_update_file,
    user_mgmt::get_user_id,
    DirectoryPolicy, DownloadIllustModes, DownloadIllustParameters,
};
use individual::illusts_to_mpsc;
use series::illusts_from_series;
use single::dl_one_illust;
use user_bookmarks::illusts_from_user_bookmarks;
use user_posts::{illusts_from_user_posts, illusts_from_user_posts_with_tag};

// -----

pub async fn download_illust(
    params: DownloadIllustParameters,
    client: SemaphoredClient,
    cookie: Option<String>,
) -> Result<()> {
    let mut internal_params = InternalDownloadParams::process_args(params, client, cookie)?;
    internal_params.download_all().await?;
    internal_params.create_update_file()?;

    Ok(())
}

// -----

// TODO: I don't really like that struct after all... Maybe a more linear way of doing things is better

struct InternalDownloadParams {
    /// Parameters for querying API and get illust ids
    source: DownloadSource,

    client: SemaphoredClient,

    /// Used for deduplication
    file_list: Option<Arc<Vec<String>>>,

    /// (1) Where all illusts will end up under
    base_dest: PathBuf,
    /// (2) When downloading a collection of illusts (e.g. series), should a new directory named after the collection be created
    create_named_dir: bool,
    /// (3) If each illust will have its own subdir or not
    directory_policy: DirectoryPolicy,

    /// Path where illusts will be downloaded, i.e., base_dest and named dir joined after name of collection was retrieved
    dest_dir: Option<PathBuf>,

    /// After download is complete, create an update file ?
    make_update_file: bool,
}

impl InternalDownloadParams {
    /// Will take arguments given by user from CLI, process them for download
    fn process_args(
        params: DownloadIllustParameters,
        client: SemaphoredClient,
        cookie: Option<String>,
    ) -> Result<InternalDownloadParams> {
        // If there is a specified path, use it, otherwise use blank for current dir
        let base_dest = params.output_directory.clone().unwrap_or(current_dir()?);

        // If incremental is active, list all files
        let file_list = if let Some(o) = &params.incremental {
            Some(Arc::new(list_all_files(o.as_ref().unwrap_or(&base_dest))?))
        } else {
            None
        };

        // Check if we're going to create a named sub directory
        let create_named_dir =
            should_create_named_dir(params.disable_named_dir, &params.mode, &base_dest)?;

        // Should we create an update file
        let make_update_file = !params.no_update_file & params.incremental.is_none();

        // Process arguments
        let mode = DownloadSource::from_args(params.mode, cookie)?;

        Ok(InternalDownloadParams {
            source: mode,
            client,
            file_list,
            base_dest,
            create_named_dir,
            directory_policy: params.directory_policy,
            dest_dir: None,
            make_update_file,
        })
    }

    /// Download all files
    async fn download_all(&mut self) -> Result<()> {
        // Channel for passing illust ids
        let (illust_id_tx, illust_id_rx) = mpsc::unbounded_channel();

        // Channel for propagating name of collection
        let (name_tx, name_rx) = if self.create_named_dir {
            let channel = oneshot::channel();
            (Some(channel.0), Some(channel.1))
        } else {
            (None, None)
        };

        // Feed MPSCs with illust ids coming from set source
        let feed_task = spawn(feed_mpsc_from_source(
            self.source.clone(),
            self.client.clone(),
            illust_id_tx,
            name_tx,
        ));

        // Get name of collection
        let name = if let Some(rx) = name_rx {
            rx.await?
        } else {
            None
        };

        // Get full dest dir, create dir if necessary
        let dest_dir = if let Some(name) = name {
            let new_path = self.base_dest.join(name);
            create_dir(&new_path).await?;
            new_path
        } else {
            self.base_dest.to_owned()
        };
        self.dest_dir = Some(dest_dir.clone());

        // Download illusts coming from MPSC
        dl_illusts_from_channel(
            self.client.clone(),
            self.directory_policy,
            illust_id_rx,
            dest_dir,
            self.file_list.clone(),
        )
        .await?;
        feed_task.await??;

        Ok(())
    }

    fn create_update_file(&self) -> Result<()> {
        // Check if prevented by arguments
        if !self.make_update_file {
            return Ok(());
        }

        // Only keep collections as they're the only ones that may get updated later
        if !self.source.is_collection() {
            return Ok(());
        }

        let arg = self.source.to_arg();

        let dest_dir = self.dest_dir.clone().unwrap_or(self.base_dest.clone());

        create_update_file(&dest_dir, &arg)
    }
}

#[derive(Clone)]
enum DownloadSource {
    Individual {
        illust_ids: Vec<u64>,
    },
    Series {
        series_id: u64,
    },
    UserPosts {
        user_id: u64,
    },
    UserPostsTag {
        user_id: u64,
        tag: String,
    },
    UserBookmarks {
        user_id: u64,
    },
    OwnBookmarks {
        user_id: u64,
        public: bool,
        private: bool,
    },
}

impl DownloadSource {
    fn from_args(args: DownloadIllustModes, cookie: Option<String>) -> Result<DownloadSource> {
        Ok(match args {
            DownloadIllustModes::Individual { illust_ids } => {
                DownloadSource::Individual { illust_ids }
            }
            DownloadIllustModes::Series { series_id } => DownloadSource::Series { series_id },
            DownloadIllustModes::UserPosts { tag, user_id } => {
                if let Some(tag) = tag {
                    DownloadSource::UserPostsTag { user_id, tag }
                } else {
                    DownloadSource::UserPosts { user_id }
                }
            }
            DownloadIllustModes::UserBookmarks { user_id } => {
                DownloadSource::UserBookmarks { user_id }
            }
            DownloadIllustModes::OwnBookmarks { public, private } => {
                if !public & !private {
                    return Err(anyhow::anyhow!(
                        "Neither Public nor Private was selected for download !"
                    ));
                }

                if let Some(c) = cookie {
                    if let Some(i) = get_user_id(&c) {
                        // Extracted from the cookie
                        DownloadSource::OwnBookmarks {
                            user_id: i,
                            public,
                            private,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Couldn't get user id from cookie !"));
                    }
                } else {
                    return Err(anyhow::anyhow!("No user cookie available !"));
                }
            }
        })
    }

    /// Convert into a arg-like struct containing exact parameters used during download
    fn to_arg(&self) -> DownloadIllustModes {
        match self {
            DownloadSource::Individual { illust_ids } => DownloadIllustModes::Individual {
                illust_ids: illust_ids.clone(),
            },
            DownloadSource::Series { series_id } => DownloadIllustModes::Series {
                series_id: *series_id,
            },
            DownloadSource::UserPosts { user_id } => DownloadIllustModes::UserPosts {
                tag: None,
                user_id: *user_id,
            },
            DownloadSource::UserPostsTag { user_id, tag } => DownloadIllustModes::UserPosts {
                tag: Some(tag.clone()),
                user_id: *user_id,
            },
            DownloadSource::UserBookmarks { user_id } => {
                DownloadIllustModes::UserBookmarks { user_id: *user_id }
            }
            DownloadSource::OwnBookmarks {
                user_id: _,
                public,
                private,
            } => DownloadIllustModes::OwnBookmarks {
                public: *public,
                private: *private,
            },
        }
    }

    fn is_collection(&self) -> bool {
        !matches!(self, DownloadSource::Individual { illust_ids: _ })
    }
}

// -----

/// Get illust ids from API and feed them to MPSC for download
async fn feed_mpsc_from_source(
    source: DownloadSource,
    client: SemaphoredClient,
    illust_id_tx: UnboundedSender<u64>,
    name_tx: Option<Sender<Option<String>>>,
) -> Result<()> {
    match source {
        DownloadSource::Individual { illust_ids } => {
            if let Some(tx) = name_tx {
                tx.send(None)
                    .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            }
            illusts_to_mpsc(&illust_ids, illust_id_tx).await?
        }
        DownloadSource::Series { series_id } => {
            illusts_from_series(client, series_id, illust_id_tx, name_tx).await?
        }
        DownloadSource::UserPosts { user_id } => {
            if let Some(tx) = name_tx {
                tx.send(None)
                    .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            }
            illusts_from_user_posts(client, user_id, illust_id_tx).await?
        }
        DownloadSource::UserPostsTag { user_id, tag } => {
            if let Some(tx) = name_tx {
                tx.send(None)
                    .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            }
            illusts_from_user_posts_with_tag(client, user_id, &tag, illust_id_tx).await?
        }
        DownloadSource::UserBookmarks { user_id } => {
            if let Some(tx) = name_tx {
                tx.send(None)
                    .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            }
            illusts_from_user_bookmarks(client, user_id, illust_id_tx, true, false).await?
        }
        DownloadSource::OwnBookmarks {
            user_id,
            public,
            private,
        } => {
            if let Some(tx) = name_tx {
                tx.send(None)
                    .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            }
            illusts_from_user_bookmarks(client, user_id, illust_id_tx, public, private).await?
        }
    }

    Ok(())
}

/// Initiates illust downloads coming from MPSC channel
async fn dl_illusts_from_channel(
    client: SemaphoredClient,
    directory_policy: DirectoryPolicy,
    mut illust_rx: UnboundedReceiver<u64>,
    dest_dir: PathBuf,
    file_list: Option<Arc<Vec<String>>>,
) -> Result<()> {
    let mut set = JoinSet::new();

    // For all received illust ids
    while let Some(illust_id) = illust_rx.recv().await {
        if let Some(fl) = &file_list {
            // If there is a file list, check duplicates before dl
            set.spawn(check_dup_and_dl(
                client.clone(),
                illust_id,
                dest_dir.clone(),
                directory_policy,
                fl.clone(),
            ));
        } else {
            // Otherwise, directly perform download
            set.spawn(dl_one_illust(
                client.clone(),
                illust_id,
                dest_dir.clone(),
                directory_policy,
            ));
        }
    }

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
}

/// Checks if an illust is already in destination path and only download if not found
async fn check_dup_and_dl(
    client: SemaphoredClient,
    illust_id: u64,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Arc<Vec<String>>,
) -> Result<()> {
    // Check if file is already downloaded
    if is_illust_in_files(&illust_id.to_string(), &file_list) {
        return Ok(());
    }

    // Proceed to download
    dl_one_illust(client, illust_id, dest_dir, directory_policy).await
}

/// Checks if it would be wise to create a new directory named after series or user within specified destination directory
fn should_create_named_dir(
    creation_disabled: bool,
    dl_mode: &DownloadIllustModes,
    dest_dir: &Path,
) -> Result<bool> {
    // Creation is straight up disabled
    if creation_disabled {
        return Ok(false);
    }

    // Only create in specific modes
    match dl_mode {
        // For now, only in series
        DownloadIllustModes::Series { series_id: _ } => {}
        _ => return Ok(false),
    }

    // Check contents of dest dir
    let (_, nb_dirs) = count_files_dirs(dest_dir)?;

    // If dir is empty or only contains files, assume user wants illusts directly in this dir.
    if nb_dirs == 0 {
        return Ok(false);
    }

    // At least 1 other dir, assume user wants new named dir in dest
    Ok(true)
}

/// Count nb of files and dirs in specified dir
pub fn count_files_dirs(path: &Path) -> Result<(usize, usize)> {
    if !path.is_dir() {
        return Err(anyhow!("Not a dir"));
    }

    let mut nb_files = 0;
    let mut nb_dirs = 0;

    for entry in read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            nb_files += 1;
        } else if entry_path.is_dir() {
            nb_dirs += 1;
        }
    }

    Ok((nb_files, nb_dirs))
}
