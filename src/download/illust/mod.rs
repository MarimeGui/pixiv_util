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
    spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
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
use single::{dl_one_illust, IllustDownload};
use user_bookmarks::illusts_from_user_bookmarks;
use user_posts::{illusts_from_user_posts, illusts_from_user_posts_with_tag};

// TODO: Update file is always created in base_dir, should take into account named dir
// TODO: Named dir should be propagated using watch then.

// -----

pub async fn download_illust(
    params: DownloadIllustParameters,
    client: SemaphoredClient,
    cookie: Option<String>,
) -> Result<()> {
    let internal_params = InternalDownloadParams::process_args(params, client, cookie)?;
    internal_params.download_all().await?;
    internal_params.create_update_file()?;

    Ok(())
}

// -----

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
            create_named_dir(params.disable_named_dir, &params.mode, &base_dest)?;

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
            make_update_file,
        })
    }

    /// Download all files
    async fn download_all(&self) -> Result<()> {
        // Create MPSC channel for illust ids and spawn task to begin download
        let (illust_tx, illust_rx) = unbounded_channel();
        let illust_result = spawn(dl_illusts_from_channel(
            self.client.clone(),
            self.directory_policy,
            illust_rx,
            self.file_list.clone(),
        ));

        // Secondary MPSC channel that includes default destination dir if it goes unchanged
        let (def_path_illust_tx, def_path_illust_rx) = unbounded_channel();
        let adder_result = spawn(add_default_path(
            def_path_illust_rx,
            illust_tx.clone(),
            self.base_dest.clone(),
        ));

        // Feed MPSCs with illust ids coming from set source
        self.feed_mpsc_from_source(illust_tx.clone(), def_path_illust_tx.clone())
            .await?;

        // Check adder did not error
        drop(def_path_illust_tx);
        adder_result.await??;

        // Check all illusts were downloaded properly
        drop(illust_tx);
        illust_result.await??;

        Ok(())
    }

    /// Get illust ids from API and feed them to MPSC for download
    async fn feed_mpsc_from_source(
        &self,
        illust_tx: UnboundedSender<IllustDownload>,
        def_path_illust_tx: UnboundedSender<u64>,
    ) -> Result<()> {
        match &self.source {
            DownloadSource::Individual { illust_ids } => {
                illusts_to_mpsc(illust_ids, def_path_illust_tx).await?
            }
            DownloadSource::Series { series_id } => {
                illusts_from_series(
                    self.client.clone(),
                    self.base_dest.clone(),
                    self.create_named_dir,
                    *series_id,
                    illust_tx,
                )
                .await?
            }
            DownloadSource::UserPosts { user_id } => {
                illusts_from_user_posts(self.client.clone(), *user_id, def_path_illust_tx).await?
            }
            DownloadSource::UserPostsTag { user_id, tag } => {
                illusts_from_user_posts_with_tag(
                    self.client.clone(),
                    *user_id,
                    tag,
                    def_path_illust_tx,
                )
                .await?
            }
            DownloadSource::UserBookmarks { user_id } => {
                illusts_from_user_bookmarks(self.client.clone(), *user_id, def_path_illust_tx)
                    .await?
            }
        }

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

        create_update_file(&self.base_dest, &arg)
    }
}

enum DownloadSource {
    Individual { illust_ids: Vec<u64> },
    Series { series_id: u64 },
    UserPosts { user_id: u64 },
    UserPostsTag { user_id: u64, tag: String },
    UserBookmarks { user_id: u64 },
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
                if let Some(user_id) = user_id {
                    // Specified directly by command line
                    DownloadSource::UserBookmarks { user_id }
                } else if let Some(c) = cookie {
                    if let Some(i) = get_user_id(&c) {
                        // Extracted from the cookie
                        DownloadSource::UserBookmarks { user_id: i }
                    } else {
                        return Err(anyhow::anyhow!("Couldn't get user id from cookie !"));
                    }
                } else {
                    return Err(anyhow::anyhow!("No user ID specified !"));
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
            DownloadSource::UserBookmarks { user_id } => DownloadIllustModes::UserBookmarks {
                user_id: Some(*user_id),
            },
        }
    }

    fn is_collection(&self) -> bool {
        !matches!(self, DownloadSource::Individual { illust_ids: _ })
    }
}

// -----

async fn add_default_path(
    mut def_path_illust_rx: UnboundedReceiver<u64>,
    illust_tx: UnboundedSender<IllustDownload>,
    default_path: PathBuf,
) -> Result<()> {
    while let Some(id) = def_path_illust_rx.recv().await {
        illust_tx.send(IllustDownload {
            id,
            dest_dir: default_path.clone(),
        })?;
    }

    Ok(())
}

/// Initiates illust downloads coming from MPSC channel
async fn dl_illusts_from_channel(
    client: SemaphoredClient,
    directory_policy: DirectoryPolicy,
    mut illust_rx: UnboundedReceiver<IllustDownload>,
    file_list: Option<Arc<Vec<String>>>,
) -> Result<()> {
    let mut set = JoinSet::new();

    // For all received illust ids
    while let Some(illust) = illust_rx.recv().await {
        if let Some(fl) = file_list.clone() {
            // If there is a file list, check duplicates before dl
            set.spawn(check_dup_and_dl(
                client.clone(),
                illust,
                directory_policy,
                fl,
            ));
        } else {
            // Otherwise, directly perform download
            set.spawn(dl_one_illust(client.clone(), illust, directory_policy));
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
    illust: IllustDownload,
    directory_policy: DirectoryPolicy,
    file_list: Arc<Vec<String>>,
) -> Result<()> {
    // Check if file is already downloaded
    if is_illust_in_files(&illust.id.to_string(), &file_list) {
        return Ok(());
    }

    // Proceed to download
    dl_one_illust(client, illust, directory_policy).await
}

/// Checks if it would be wise to create a new directory named after series or user within specified destination directory
fn create_named_dir(
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
