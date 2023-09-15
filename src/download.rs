use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use tokio_stream::StreamExt;

use crate::{
    abstractions::{get_all_series_works, get_all_user_bookmarks, get_all_user_img_posts},
    gen_http_client::{make_client, make_headers},
    incremental::{is_illust_in_files, list_all_files},
    user_mgmt::{get_user_id, retrieve_cookie},
    DirectoryPolicy, DownloadModesSubcommands, DownloadParameters,
};

// -------

pub async fn do_download_subcommand(params: DownloadParameters) -> Result<()> {
    // Get a cookie, if any
    let cookie = match params.cookie_override {
        Some(c) => Some(c),
        None => retrieve_cookie(params.user_override).await?,
    };

    // Make the HTTP client with correct headers
    let client = make_client(make_headers(cookie.as_deref())?)?;

    // If there is a specified path, use it, otherwise use blank for current dir
    let output_dir = params.output_directory.unwrap_or_default();

    // If incremental is active, list all files
    let file_list = if let Some(o) = params.incremental {
        Some(list_all_files(o.unwrap_or(output_dir.clone()))?)
    } else {
        None
    };

    // Closure for initiating downloads
    let mut tasks = Vec::new();
    let mut f = |illust_id: u64| {
        // If this ID is already found among files, don't download it
        if let Some(l) = &file_list {
            // TODO: We are probably loosing a bit of performance by computing here
            if is_illust_in_files(&illust_id.to_string(), l) {
                // If we already have this illust, signal that this does not need to be downloaded
                // If this is reached and fast incremental is disabled, this will return true and signal to continue
                return params.disable_fast_incremental;
            }
        }
        let client = client.clone();
        let save_path = output_dir.clone();
        tasks.push(tokio::spawn(async move {
            dl_illust(&client, illust_id, save_path, params.directory_policy).await
        }));
        true
    };

    // Run all tasks
    match params.mode {
        DownloadModesSubcommands::Individual { illust_ids } => {
            for illust_id in illust_ids {
                f(illust_id);
            }
        }
        DownloadModesSubcommands::Series { series_id } => {
            get_all_series_works(&client, series_id, f).await?
        }
        DownloadModesSubcommands::UserPosts { user_id } => {
            get_all_user_img_posts(&client, user_id, f).await?;
        }
        DownloadModesSubcommands::UserBookmarks { user_id } => {
            // Get user id to use for downloads
            let id = if let Some(i) = user_id {
                // Specified directly by command line
                i
            } else if let Some(c) = cookie {
                if let Some(i) = get_user_id(&c) {
                    // Extracted from the cookie
                    i
                } else {
                    return Err(anyhow::anyhow!("Couldn't get user id from cookie !"));
                }
            } else {
                return Err(anyhow::anyhow!("No user ID specified !"));
            };

            get_all_user_bookmarks(&client, id, f).await?;
        }
    }

    // Check if every download went okay
    for task in tasks {
        task.await??;
    }

    Ok(())
}

// -------

const MAX_RETRIES: usize = 3;
const TIMEOUT: u64 = 120;

struct DownloadTask<T> {
    url: String,
    path: PathBuf,
    tries: usize,
    task: Option<T>,
}

pub async fn dl_illust(
    client: &Client,
    illust_id: u64,
    mut save_path: PathBuf,
    directory_policy: DirectoryPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust::get(client, illust_id).await?;

    let in_dir = match directory_policy {
        DirectoryPolicy::AlwaysCreate => true,
        DirectoryPolicy::NeverCreate => false,
        DirectoryPolicy::CreateIfMultiple => pages.len() > 1,
    };

    // If multiple pages, put everything in dir
    if in_dir {
        create_dir_all(illust_id.to_string()).await?;
        save_path.push(illust_id.to_string());
    }

    let mut downloads = Vec::with_capacity(pages.len());
    for page in pages.into_iter() {
        let url = page.urls.original;

        // Extract filename
        let filename = {
            match url.rsplit('/').next() {
                Some(p) => p,
                None => &url,
            }
        };

        // Append filename
        let mut save_path = save_path.clone();
        save_path.push(filename);

        downloads.push(DownloadTask {
            url,
            path: save_path,
            tries: 0,
            task: None,
        })
    }

    // Check on every task, retry if necessary
    loop {
        let mut done = true;

        for download in downloads.iter_mut() {
            let initiate = match (download.tries, &mut download.task) {
                // Hasn't yet started
                (0, None) => true,
                // Has completed already
                (_, None) => false,
                // Is undergoing
                (current_tries, Some(j)) => match j.await {
                    Ok(i) => match i {
                        // We're done !
                        Ok(_) => false,
                        Err(e) => {
                            if current_tries > MAX_RETRIES {
                                // Tried too many times, let it go.
                                eprintln!("    {}", e);
                                false
                            } else {
                                // Try again !
                                true
                            }
                        }
                    },
                    // JoinError, print error and don't try again.
                    Err(e) => {
                        eprintln!("    {}", e);
                        false
                    }
                },
            };

            download.task = if initiate {
                done = false;
                download.tries += 1;
                let req = client
                    .get(&download.url)
                    .timeout(Duration::from_secs(TIMEOUT));
                let path_clone = download.path.clone();
                Some(tokio::spawn(dl_image_to_disk(path_clone, req)))
            } else {
                None
            };
        }

        if done {
            break;
        }
    }

    Ok(())
}

async fn dl_image_to_disk(save_path: PathBuf, req: RequestBuilder) -> Result<()> {
    let resp = req.send().await?;
    resp.error_for_status_ref()?;

    let mut file = File::create(save_path).await?;
    let mut stream = resp.bytes_stream();

    while let Some(data) = stream.next().await {
        file.write_all(&data?).await?
    }

    Ok(())
}
