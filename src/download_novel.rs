use anyhow::Error;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    gen_http_client::{make_client, make_headers},
    user_mgmt::retrieve_cookie,
    DownloadNovelParameters,
};

pub async fn do_download_novel_subcommand(params: DownloadNovelParameters) -> Result<(), Error> {
    let cookie = match params.cookie_override {
        Some(c) => Some(c),
        None => retrieve_cookie(params.user_override).await?,
    };

    let client = make_client(make_headers(cookie.as_deref())?)?;

    let info = crate::api_calls::novel::get(&client, params.novel_id).await?;

    let mut file = File::create(params.destination_file).await?;
    file.write_all(info.content.as_bytes()).await?;

    Ok(())
}
