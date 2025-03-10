use anyhow::Result;
use tokio::{spawn, sync::mpsc::UnboundedSender, task::JoinSet};

use crate::{api_calls::user_bookmarks::Visibility, gen_http_client::SemaphoredClient};

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

/// Will download all illusts bookmarked by specified user
pub async fn illusts_from_user_bookmarks(
    client: SemaphoredClient,
    user_id: u64,
    illust_tx: UnboundedSender<u64>,
    public: bool,
    private: bool,
) -> Result<()> {
    let private_task = if private {
        Some(spawn(dl_bookmarks(
            client.clone(),
            user_id,
            illust_tx.clone(),
            Visibility::Private,
        )))
    } else {
        None
    };
    let public_task = if public {
        Some(spawn(dl_bookmarks(
            client.clone(),
            user_id,
            illust_tx.clone(),
            Visibility::Public,
        )))
    } else {
        None
    };

    if let Some(j) = private_task {
        j.await??
    }
    if let Some(j) = public_task {
        j.await??
    }

    Ok(())
}

async fn dl_bookmarks(
    client: SemaphoredClient,
    user_id: u64,
    illust_tx: UnboundedSender<u64>,
    visibility: Visibility,
) -> Result<()> {
    // Fetch first bookmark page to get total amount of illusts
    let nb_illusts =
        dl_one_bookmark_page(client.clone(), illust_tx.clone(), user_id, 0, visibility).await?;

    // Assume page will always contain max except for last one, calc number of pages
    let page_count =
        nb_illusts / ILLUSTS_PER_PAGE + usize::from(nb_illusts % ILLUSTS_PER_PAGE != 0);

    // Fetch all other bookmark pages
    let mut set = JoinSet::new();
    for page_id in 1..page_count {
        set.spawn(dl_one_bookmark_page(
            client.clone(),
            illust_tx.clone(),
            user_id,
            page_id * ILLUSTS_PER_PAGE,
            visibility,
        ));
    }
    drop(illust_tx);
    while let Some(r) = set.join_next().await {
        r??;
    }

    Ok(())
}

/// Acquires and filters contents from API
async fn dl_one_bookmark_page(
    client: SemaphoredClient,
    illust_tx: UnboundedSender<u64>,
    user_id: u64,
    offset: usize,
    visibility: Visibility,
) -> Result<usize> {
    let body = crate::api_calls::user_bookmarks::get(
        client.clone(),
        user_id,
        offset,
        ILLUSTS_PER_PAGE,
        visibility,
    )
    .await?;

    for work in &body.works {
        // Ignore illusts that have been removed
        if work.is_masked {
            continue;
        }

        illust_tx.send(work.id)?;
    }

    Ok(body.total)
}
