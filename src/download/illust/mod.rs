mod individual;
mod series;
mod single;
mod user_bookmarks;
mod user_posts;

use anyhow::Result;
use individual::dl_individual;
use series::dl_series;
use user_bookmarks::dl_user_bookmarks;
use user_posts::{dl_user_posts, dl_user_posts_with_tag};

use crate::{
    gen_http_client::SemaphoredClient, incremental::list_all_files,
    update_file::create_update_file, user_mgmt::get_user_id, DownloadIllustModes,
    DownloadIllustParameters,
};

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

    match &params.mode {
        DownloadIllustModes::Individual { illust_ids } => {
            dl_individual(
                client,
                dest_dir.clone(),
                params.directory_policy,
                illust_ids,
            )
            .await?
        }
        DownloadIllustModes::Series { series_id } => {
            dl_series(
                client,
                dest_dir.clone(),
                params.directory_policy,
                file_list,
                *series_id,
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
                    *user_id,
                    tag,
                )
                .await?
            }
            None => {
                dl_user_posts(
                    client,
                    dest_dir.clone(),
                    params.directory_policy,
                    file_list,
                    *user_id,
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
                params.directory_policy,
                file_list,
                id,
            )
            .await?
        }
    }

    // Create an update file
    if !params.no_update_file & params.incremental.is_none() {
        match &params.mode {
            // Ignore single illusts
            DownloadIllustModes::Individual { illust_ids: _ } => {}
            _ => {
                create_update_file(&dest_dir, &params.mode)?;
            }
        }
    }

    Ok(())
}
