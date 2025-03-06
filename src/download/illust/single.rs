use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use tokio::{fs::create_dir_all, task::JoinSet};

use crate::{download::file::safe_dl, gen_http_client::SemaphoredClient, DirectoryPolicy};

const MAX_RETRIES: usize = 3;
const TIMEOUT: u64 = 120;

/// Information required for downloading an illust
pub struct IllustDownload {
    /// ID of the illust to download
    pub id: u64,
    /// Destination dir of illust
    pub dest_dir: PathBuf,
}

pub async fn dl_one_illust(
    client: SemaphoredClient,
    mut illust: IllustDownload,
    directory_policy: DirectoryPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust_pages::get(client.clone(), illust.id).await?;

    let in_dir = match directory_policy {
        DirectoryPolicy::AlwaysCreate => true,
        DirectoryPolicy::NeverCreate => false,
        DirectoryPolicy::CreateIfMultiple => pages.len() > 1,
    };

    // If multiple pages, put everything in dir
    if in_dir {
        create_dir_all(illust.id.to_string()).await?;
        illust.dest_dir.push(illust.id.to_string());
    }

    let mut set = JoinSet::new();

    // Initiate all downloads
    for page in pages.into_iter() {
        set.spawn(safe_dl(
            client.clone(),
            page.urls.original,
            illust.dest_dir.clone(),
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
