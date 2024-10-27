use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use tokio::{fs::create_dir_all, task::JoinSet};

use crate::{
    abstractions::{
        get_all_series_works, get_all_user_bookmarks, get_all_user_img_posts,
        get_all_user_img_posts_with_tag,
    },
    file_download::safe_dl,
    gen_http_client::SemaphoredClient,
    incremental::{is_illust_in_files, list_all_files},
    update_file::create_update_file,
    user_mgmt::get_user_id,
    DirectoryPolicy, DownloadIllustModes, DownloadIllustParameters,
};

// -----

const MAX_RETRIES: usize = 3;
const TIMEOUT: u64 = 120;

// -----

pub async fn download_illust(
    params: DownloadIllustParameters,
    client: SemaphoredClient,
    cookie: Option<String>,
) -> Result<()> {
    // If there is a specified path, use it, otherwise use blank for current dir
    let output_dir = params.output_directory.unwrap_or_default();

    // If incremental is active, list all files
    let file_list = if let Some(o) = &params.incremental {
        Some(list_all_files(o.as_ref().unwrap_or(&output_dir))?)
    } else {
        None
    };

    // Closure for initiating downloads
    let mut tasks = Vec::new();
    let mut f = |illust_id: u64| {
        // If this ID is already found among files, don't download it
        if let Some(l) = &file_list {
            // TODO: We are probably loosing a bit of performance by computing here
            if is_illust_in_files(&illust_id.to_string(), l) {
                // If we already have this illust, signal that this does not need to be downloaded
                // If this is reached and fast incremental is disabled, this will return true and signal to continue
                return !params.fast_incremental;
            }
        }
        let client = client.clone();
        let save_path = output_dir.clone();
        tasks.push(tokio::spawn(async move {
            dl_illust(client, illust_id, save_path, params.directory_policy).await
        }));
        true
    };

    // Run all tasks
    match &params.mode {
        DownloadIllustModes::Individual { illust_ids } => {
            for illust_id in illust_ids {
                f(*illust_id);
            }
        }
        DownloadIllustModes::Series { series_id } => {
            get_all_series_works(client.clone(), *series_id, f).await?
        }
        DownloadIllustModes::UserPosts { tag, user_id } => {
            if let Some(tag) = tag {
                get_all_user_img_posts_with_tag(client.clone(), *user_id, tag, f).await?;
            } else {
                get_all_user_img_posts(client.clone(), *user_id, f).await?;
            }
        }
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

            get_all_user_bookmarks(client.clone(), id, f).await?;
        }
    }

    // Check if every illust download went okay
    for task in tasks {
        task.await??;
    }

    // Create an update file
    if !params.no_update_file & params.incremental.is_none() {
        match &params.mode {
            // Ignore single illusts
            DownloadIllustModes::Individual { illust_ids: _ } => {}
            _ => {
                create_update_file(&output_dir, &params.mode)?;
            }
        }
    }

    Ok(())
}

pub async fn dl_illust(
    client: SemaphoredClient,
    illust_id: u64,
    mut dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust_pages::get(client.clone(), illust_id).await?;

    let in_dir = match directory_policy {
        DirectoryPolicy::AlwaysCreate => true,
        DirectoryPolicy::NeverCreate => false,
        DirectoryPolicy::CreateIfMultiple => pages.len() > 1,
    };

    // If multiple pages, put everything in dir
    if in_dir {
        create_dir_all(illust_id.to_string()).await?;
        dest_dir.push(illust_id.to_string());
    }

    let mut set = JoinSet::new();

    // Initiate all downloads
    for page in pages.into_iter() {
        set.spawn(safe_dl(
            client.clone(),
            page.urls.original,
            dest_dir.clone(),
            MAX_RETRIES,
            Duration::from_secs(TIMEOUT),
        ));
    }

    // Wait for completion of all downloads
    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
}
