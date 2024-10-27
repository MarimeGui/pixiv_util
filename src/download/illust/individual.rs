use std::path::PathBuf;

use anyhow::Result;
use tokio::task::JoinSet;

use crate::{gen_http_client::SemaphoredClient, DirectoryPolicy};

use super::single::dl_one_illust;

pub async fn dl_individual(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    illust_ids: &[u64],
) -> Result<()> {
    let mut set = JoinSet::new();

    for illust_id in illust_ids {
        set.spawn(dl_one_illust(
            client.clone(),
            *illust_id,
            dest_dir.clone(),
            directory_policy,
        ));
    }

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
}
