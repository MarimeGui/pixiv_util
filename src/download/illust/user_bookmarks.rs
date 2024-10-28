use std::{path::PathBuf, sync::Arc};

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
    // Arc for file list to prevent useless copies
    let file_list = Arc::new(file_list);

    // First DL to get total number of illusts
    let nb_illusts = dl_one_bookmark_page(
        client.clone(),
        dest_dir.clone(),
        directory_policy,
        file_list.clone(),
        user_id,
        0,
    )
    .await?;

    // Assume page will always contain max except for last one, calc number of pages
    let page_count =
        nb_illusts / ILLUSTS_PER_PAGE + usize::from(nb_illusts % ILLUSTS_PER_PAGE != 0);

    // Start all pages
    let mut set = JoinSet::new();
    for page_id in 1..page_count {
        set.spawn(dl_one_bookmark_page(
            client.clone(),
            dest_dir.clone(),
            directory_policy,
            file_list.clone(),
            user_id,
            page_id * ILLUSTS_PER_PAGE,
        ));
    }

    while let Some(r) = set.join_next().await {
        r??;
    }

    Ok(())
}

async fn dl_one_bookmark_page(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Arc<Option<Vec<String>>>,
    user_id: u64,
    offset: usize,
) -> Result<usize> {
    let mut set = JoinSet::new();

    let body =
        crate::api_calls::user_bookmarks::get(client.clone(), user_id, offset, ILLUSTS_PER_PAGE)
            .await?;

    for work in &body.works {
        // Check if file already downloaded
        if let Some(files) = &*file_list {
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

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(body.total)
}
