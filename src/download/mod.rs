mod file;
mod illust;
mod novel;
mod update;

use anyhow::Result;

use crate::{
    gen_http_client::{make_client, make_headers},
    user_mgmt::retrieve_cookie,
    DownloadMediaParameters, DownloadParameters,
};

use self::{illust::download_illust, novel::download_novel, update::download_updates};

pub async fn do_download_subcommand(params: DownloadParameters) -> Result<()> {
    // Get a cookie, if any
    let cookie = match params.cookie_override {
        Some(c) => Some(c),
        None => retrieve_cookie(params.user_override).await?,
    };

    // Make the HTTP client with correct headers
    let client = make_client(make_headers(cookie.as_deref())?)?;

    match params.media_params {
        DownloadMediaParameters::Illust(i) => download_illust(i, client, cookie).await,
        DownloadMediaParameters::Novel(n) => download_novel(n, client).await,
        DownloadMediaParameters::Update(u) => download_updates(u, client, cookie).await,
    }
}
