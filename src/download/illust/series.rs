use std::path::PathBuf;

use anyhow::Result;
use tokio::{fs::create_dir, sync::mpsc::UnboundedSender};

use crate::gen_http_client::SemaphoredClient;

use super::IllustDownload;

pub async fn illusts_from_series(
    client: SemaphoredClient,
    mut dest_dir: PathBuf,
    mut create_named_dir: bool,
    series_id: u64,
    illust_tx: UnboundedSender<IllustDownload>,
) -> Result<()> {
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
            illust_tx.send(IllustDownload {
                id: pos.work_id,
                dest_dir: dest_dir.clone(),
            })?;
        }

        if total == body.page.total {
            break;
        }
    }

    Ok(())
}
