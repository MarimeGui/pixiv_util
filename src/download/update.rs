use std::fs::File as StdFile;

use anyhow::{anyhow, Result};

use crate::{
    gen_http_client::SemaphoredClient, update_file::UPDATE_FILE, DirectoryPolicy,
    DownloadIllustModes, DownloadIllustParameters, DownloadUpdateParameters,
};

use super::illust::download_illust;

pub async fn download_updates(
    params: DownloadUpdateParameters,
    client: SemaphoredClient,
    cookie: Option<String>,
) -> Result<()> {
    if params.recursive {
        unimplemented!()
    }

    let mut update_file_path = params.directory.clone().unwrap_or_default();
    update_file_path.push(UPDATE_FILE);

    let update_file = StdFile::open(update_file_path)
        .map_err(|e| anyhow!("Failed to open `{}` file: {}", UPDATE_FILE, e))?;

    let mode: DownloadIllustModes = serde_json::from_reader(&update_file)?;

    download_illust(
        DownloadIllustParameters {
            incremental: Some(None),
            fast_incremental: false,
            disable_named_dir: true,
            no_update_file: true,
            output_directory: params.directory,
            directory_policy: DirectoryPolicy::NeverCreate,
            mode,
        },
        client,
        cookie,
    )
    .await
}
