use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use tokio::{
    spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinSet,
};

use super::single::dl_one_illust;
use crate::{gen_http_client::SemaphoredClient, incremental::is_illust_in_files, DirectoryPolicy};

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

/// Will download all illusts bookmarked by specified user
pub async fn dl_user_bookmarks(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Option<Vec<String>>,
    user_id: u64,
) -> Result<()> {
    // Arc for file list to prevent useless copies
    let file_list = Arc::new(file_list);

    // MPSC channel for illust ids, begins download
    let (illust_tx, illust_rx) = unbounded_channel();
    let illust_result = spawn(dl_illusts_from_channel(
        client.clone(),
        dest_dir,
        directory_policy,
        illust_rx,
    ));

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
