use anyhow::Result;
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::DownloadNovelParameters;

pub async fn download_novel(params: DownloadNovelParameters, client: Client) -> Result<()> {
    let info = crate::api_calls::novel::get(&client, params.novel_id).await?;

    let mut file = File::create(params.destination_file).await?;
    file.write_all(info.content.as_bytes()).await?;

    Ok(())
}
