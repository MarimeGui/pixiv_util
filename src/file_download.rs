use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, Result};
use reqwest::RequestBuilder;
use tokio::{
    fs::{rename, File},
    io::AsyncWriteExt,
};
use tokio_stream::StreamExt;

use crate::gen_http_client::SemaphoredClient;

/// Download a file to a dir with rate limiting, timeouts, retries and temporary filenames
pub async fn safe_dl(
    client: SemaphoredClient,
    url: String,
    dest_dir: PathBuf,
    max_tries: usize,
    timeout_time: Duration,
) -> Result<()> {
    // Build all paths
    let paths = MyPaths::from_url_dest_dir(&url, &dest_dir);

    // Acquire download permit
    let permit = client.semaphore.acquire().await.unwrap(); // TODO: Handle unwrap better ?

    let mut tries = 0;

    loop {
        // Build request
        let req = client.client.get(&url).timeout(timeout_time);

        // Perform actual download
        let dl_result = dl_file_to_disk(&paths.temp, req).await;

        // Check for error
        let download_error = match dl_result {
            Ok(()) => break,
            Err(e) => e,
        };

        tries += 1;

        if tries >= max_tries {
            return Err(anyhow!(
                "'{}' failed {} times: '{}'",
                paths.filename,
                tries,
                download_error
            ));
        }
    }

    // Finished download, return permit
    drop(permit);

    // Rename from temporary filename to permanent one
    rename(paths.temp, paths.dest).await?;

    Ok(())
}

struct MyPaths {
    filename: String,
    dest: PathBuf,
    temp: PathBuf,
}

impl MyPaths {
    fn from_url_dest_dir(url: &str, dest_dir: &Path) -> MyPaths {
        // Extract filename
        let filename = {
            match url.rsplit('/').next() {
                Some(p) => p,
                None => url,
            }
        };

        // Append filename
        let mut dest = dest_dir.to_path_buf();
        let mut temp = dest.clone();
        dest.push(filename);
        temp.push(format!("._{}", filename));

        MyPaths {
            filename: filename.to_string(),
            dest,
            temp,
        }
    }
}

/// Downloads a file from a URL to specified file path
async fn dl_file_to_disk(save_path: &Path, req: RequestBuilder) -> Result<()> {
    let resp = req.send().await?;
    resp.error_for_status_ref()?;

    let mut file = File::create(&save_path).await?;
    let mut stream = resp.bytes_stream();

    while let Some(data) = stream.next().await {
        file.write_all(&data?).await?
    }

    Ok(())
}
