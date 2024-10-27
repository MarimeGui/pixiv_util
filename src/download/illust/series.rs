use std::path::PathBuf;

use anyhow::Result;
use tokio::task::JoinSet;

use crate::{gen_http_client::SemaphoredClient, incremental::is_illust_in_files, DirectoryPolicy};

use super::single::dl_one_illust;

pub async fn dl_series(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Option<Vec<String>>,
    series_id: u64,
) -> Result<()> {
    let mut set = JoinSet::new();

    let mut page_index = 1;
    let mut total = 0;

    loop {
        let body = crate::api_calls::series::get(client.clone(), series_id, page_index).await?;
        page_index += 1;

        total += body.page.series.len();

        for pos in body.page.series {
            // Check if file already downloaded
            if let Some(files) = &file_list {
                if is_illust_in_files(&pos.work_id.to_string(), files) {
                    continue;
                }
            }

            set.spawn(dl_one_illust(
                client.clone(),
                pos.work_id,
                dest_dir.clone(),
                directory_policy,
            ));
        }

        if total == body.page.total {
            break;
        }
    }

    while let Some(r) = set.join_next().await {
        r??
    }

    Ok(())
}
