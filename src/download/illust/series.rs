use anyhow::{anyhow, Result};
use tokio::sync::{mpsc::UnboundedSender, oneshot::Sender};

use crate::gen_http_client::SemaphoredClient;

pub async fn illusts_from_series(
    client: SemaphoredClient,
    series_id: u64,
    illust_id_tx: UnboundedSender<u64>,
    mut name_tx: Option<Sender<Option<String>>>,
) -> Result<()> {
    let mut page_index = 1;
    let mut total = 0;

    loop {
        let body = crate::api_calls::series::get(client.clone(), series_id, page_index).await?;
        page_index += 1;

        total += body.page.series.len();

        // Get series name
        if let Some(tx) = name_tx {
            // Try to get series info
            let to_send = if let Some(info) = body.illust_series.first() {
                // Make sure title isn't empty
                if !info.title.is_empty() {
                    Some(info.title.clone())
                } else {
                    None
                }
            } else {
                None
            };
            tx.send(to_send)
                .map_err(|e| anyhow!("Oneshot channel failed: {:?}", e))?;
            name_tx = None;
        }

        for pos in body.page.series {
            illust_id_tx.send(pos.work_id)?;
        }

        if total == body.page.total {
            break;
        }
    }

    Ok(())
}
