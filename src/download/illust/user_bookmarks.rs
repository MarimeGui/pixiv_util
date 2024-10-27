use std::path::PathBuf;

use anyhow::Result;
use tokio::task::JoinSet;

use super::single::dl_one_illust;
use crate::{gen_http_client::SemaphoredClient, incremental::is_illust_in_files, DirectoryPolicy};

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

pub async fn dl_user_bookmarks(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Option<Vec<String>>,
    user_id: u64,
) -> Result<()> {
    let mut set = JoinSet::new();

    let mut processed = 0;

    loop {
        let body = crate::api_calls::user_bookmarks::get(
            client.clone(),
            user_id,
            processed,
            ILLUSTS_PER_PAGE,
        )
        .await?;

        for work in &body.works {
            // Check if file already downloaded
            if let Some(files) = &file_list {
                if is_illust_in_files(&work.id.to_string(), files) {
                    continue;
                }
            }

            set.spawn(dl_one_illust(
                client.clone(),
                work.id,
                dest_dir.clone(),
                directory_policy,
            ));
        }

        processed += body.works.len();

        // Got every illust
        if processed >= body.total {
            break;
        }
    }

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
}
