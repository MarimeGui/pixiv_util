use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

pub async fn illusts_to_mpsc(illust_ids: &[u64], illust_tx: UnboundedSender<u64>) -> Result<()> {
    for illust_id in illust_ids {
        illust_tx.send(*illust_id)?;
    }

    Ok(())
}
