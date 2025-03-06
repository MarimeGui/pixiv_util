mod individual;
mod series;
mod single;
mod user_bookmarks;
mod user_posts;

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use tokio::{
    spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    task::JoinSet,
};

use crate::{
    gen_http_client::SemaphoredClient, incremental::list_all_files, user_mgmt::get_user_id,
    DirectoryPolicy, DownloadIllustModes, DownloadIllustParameters,
};
use individual::dl_individual;
use series::dl_series;
use single::dl_one_illust;
use user_bookmarks::dl_user_bookmarks;
use user_posts::{dl_user_posts, dl_user_posts_with_tag};

pub async fn download_illust(
    params: DownloadIllustParameters,
    client: SemaphoredClient,
    cookie: Option<String>,
) -> Result<()> {
    // If there is a specified path, use it, otherwise use blank for current dir
    let dest_dir = params.output_directory.unwrap_or_default();

    // If incremental is active, list all files
    let file_list = if let Some(o) = &params.incremental {
        Some(list_all_files(o.as_ref().unwrap_or(&dest_dir))?)
    } else {
        None
    };

    // Check if we're going to create a named sub directory
    let create_named_dir = create_named_dir(params.disable_named_dir, &params.mode, &dest_dir)?;

    // Should we create an update file
    let make_update_file = !params.no_update_file & params.incremental.is_none();

    // Create MPSC channel for illust ids and spawn task to begin download
    let (illust_tx, illust_rx) = unbounded_channel();
    let illust_result = spawn(dl_illusts_from_channel(
        client.clone(),
        dest_dir.clone(),
        params.directory_policy,
        illust_rx,
    ));

    match &params.mode {
        DownloadIllustModes::Individual { illust_ids } => {
            dl_individual(illust_ids, illust_tx).await?
        }
        DownloadIllustModes::Series { series_id } => {
            dl_series(
                client,
                dest_dir.clone(),
                file_list,
                create_named_dir,
                make_update_file,
                *series_id,
                illust_tx,
            )
            .await?
        }
        DownloadIllustModes::UserPosts { tag, user_id } => match tag {
            Some(tag) => {
                dl_user_posts_with_tag(
                    client,
                    dest_dir.clone(),
                    params.directory_policy,
                    file_list,
                    make_update_file,
                    *user_id,
                    tag,
                )
                .await?
            }
            None => {
                dl_user_posts(
                    client,
                    dest_dir.clone(),
                    file_list,
                    make_update_file,
                    *user_id,
                    illust_tx,
                )
                .await?
            }
        },
        DownloadIllustModes::UserBookmarks { user_id } => {
            // Get user id to use for downloads
            let id = if let Some(i) = user_id {
                // Specified directly by command line
                *i
            } else if let Some(c) = cookie {
                if let Some(i) = get_user_id(&c) {
                    // Extracted from the cookie
                    i
                } else {
                    return Err(anyhow::anyhow!("Couldn't get user id from cookie !"));
                }
            } else {
                return Err(anyhow::anyhow!("No user ID specified !"));
            };
            dl_user_bookmarks(
                client,
                dest_dir.clone(),
                file_list,
                make_update_file,
                id,
                illust_tx,
            )
            .await?
        }
    }

    // Check all illusts were downloaded properly
    illust_result.await??;

    Ok(())
}

/// Initiates illust downloads coming from MPSC channel
async fn dl_illusts_from_channel(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    mut illust_rx: UnboundedReceiver<u64>,
) -> Result<()> {
    let mut set = JoinSet::new();

    // For each illust
    while let Some(illust_id) = illust_rx.recv().await {
        set.spawn(dl_one_illust(
            client.clone(),
            illust_id,
            dest_dir.clone(),
            directory_policy,
        ));
    }

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
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
