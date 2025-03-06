use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use tokio::{sync::mpsc::UnboundedSender, task::JoinSet};

use crate::{
    gen_http_client::SemaphoredClient, incremental::is_illust_in_files,
    update_file::create_update_file, DownloadIllustModes,
};

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

/// Will download all illusts bookmarked by specified user
pub async fn dl_user_bookmarks(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    file_list: Option<Vec<String>>,
    make_update_file: bool,
    user_id: u64,
    illust_tx: UnboundedSender<u64>,
) -> Result<()> {
    // Arc for file list to prevent useless copies
    let file_list = Arc::new(file_list);

    // Fetch first bookmark page to get total amount of illusts
    let nb_illusts = dl_one_bookmark_page(
        client.clone(),
        file_list.clone(),
        illust_tx.clone(),
        user_id,
        0,
    )
    .await?;

    // Assume page will always contain max except for last one, calc number of pages
    let page_count =
        nb_illusts / ILLUSTS_PER_PAGE + usize::from(nb_illusts % ILLUSTS_PER_PAGE != 0);

    // Fetch all other bookmark pages
    let mut set = JoinSet::new();
    for page_id in 1..page_count {
        set.spawn(dl_one_bookmark_page(
            client.clone(),
            file_list.clone(),
            illust_tx.clone(),
            user_id,
            page_id * ILLUSTS_PER_PAGE,
        ));
    }
    drop(illust_tx);
    while let Some(r) = set.join_next().await {
        r??;
    }

    if make_update_file {
        create_update_file(
            &dest_dir,
            &DownloadIllustModes::UserBookmarks {
                user_id: Some(user_id),
            },
        )?;
    }

    Ok(())
}

/// Acquires and filters contents from API
async fn dl_one_bookmark_page(
    client: SemaphoredClient,
    file_list: Arc<Option<Vec<String>>>,
    illust_tx: UnboundedSender<u64>,
    user_id: u64,
    offset: usize,
) -> Result<usize> {
    let body =
        crate::api_calls::user_bookmarks::get(client.clone(), user_id, offset, ILLUSTS_PER_PAGE)
            .await?;

    for work in &body.works {
        // Ignore illusts that have been removed
        if work.is_masked {
            continue;
        }

        // Check if file already downloaded
        if let Some(files) = &*file_list {
            if is_illust_in_files(&work.id.to_string(), files) {
                continue;
            }
        }

        illust_tx.send(work.id)?;
    }

    Ok(body.total)
}
