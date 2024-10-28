use std::path::PathBuf;

use anyhow::Result;
use tokio::{fs::create_dir, task::JoinSet};

use crate::{
    gen_http_client::SemaphoredClient, incremental::is_illust_in_files,
    update_file::create_update_file, DirectoryPolicy, DownloadIllustModes,
};

use super::single::dl_one_illust;

pub async fn dl_series(
    client: SemaphoredClient,
    mut dest_dir: PathBuf,
    directory_policy: DirectoryPolicy,
    file_list: Option<Vec<String>>,
    mut create_named_dir: bool,
    make_update_file: bool,
    series_id: u64,
) -> Result<()> {
    let mut set = JoinSet::new();

    let mut page_index = 1;
    let mut total = 0;

    loop {
        let body = crate::api_calls::series::get(client.clone(), series_id, page_index).await?;
        page_index += 1;

        total += body.page.series.len();

        // Modify dest path if required
        if create_named_dir {
            // If no series info for some reason, fail silently
            if let Some(info) = body.illust_series.first() {
                // Make sure title isn't empty
                if !info.title.is_empty() {
                    // Append dir
                    dest_dir.push(info.title.clone());
                    // Create dir
                    create_dir(&dest_dir).await?;
                }
            }
            create_named_dir = false;
        }

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

    if make_update_file {
        create_update_file(&dest_dir, &DownloadIllustModes::Series { series_id })?;
    }

    Ok(())
}
