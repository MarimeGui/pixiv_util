use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use tokio::{fs::create_dir_all, task::JoinSet};

use crate::{download::file::safe_dl, gen_http_client::SemaphoredClient, DirectoryPolicy};

const MAX_RETRIES: usize = 3;
const TIMEOUT: u64 = 120;

pub async fn dl_one_illust(
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
