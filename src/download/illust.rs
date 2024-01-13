use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use tokio::{
    fs::{create_dir_all, rename, File},
    io::AsyncWriteExt,
    task::JoinHandle,
};
use tokio_stream::StreamExt;

use crate::{
    abstractions::{get_all_series_works, get_all_user_bookmarks, get_all_user_img_posts},
    incremental::{is_illust_in_files, list_all_files},
    update_file::create_update_file,
    user_mgmt::get_user_id,
    DirectoryPolicy, DownloadIllustModes, DownloadIllustParameters,
};

// -----

const MAX_RETRIES: usize = 3;
const TIMEOUT: u64 = 120;
const MAX_PARALLEL: usize = 100;

// -----

pub async fn download_illust(
    params: DownloadIllustParameters,
    client: Client,
    cookie: Option<String>,
) -> Result<()> {
    // If there is a specified path, use it, otherwise use blank for current dir
    let output_dir = params.output_directory.unwrap_or_default();

    // If incremental is active, list all files
    let file_list = if let Some(o) = &params.incremental {
        Some(list_all_files(o.as_ref().unwrap_or(&output_dir))?)
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
                return !params.fast_incremental;
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
    match &params.mode {
        DownloadIllustModes::Individual { illust_ids } => {
            for illust_id in illust_ids {
                f(*illust_id);
            }
        }
        DownloadIllustModes::Series { series_id } => {
            get_all_series_works(&client, *series_id, f).await?
        }
        DownloadIllustModes::UserPosts { user_id } => {
            get_all_user_img_posts(&client, *user_id, f).await?;
        }
        DownloadIllustModes::UserBookmarks { user_id } => {
            // Get user id to use for downloads
            let id = if let Some(i) = user_id {
                // Specified directly by command line
                *i
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

    // Check if every illust download went okay
    for task in tasks {
        task.await??;
    }

    // Create an update file
    if !params.no_update_file & params.incremental.is_none() {
        match &params.mode {
            // Ignore single illusts
            DownloadIllustModes::Individual { illust_ids: _ } => {}
            _ => {
                create_update_file(&output_dir, &params.mode)?;
            }
        }
    }

    Ok(())
}

struct DownloadInfo {
    url: String,
    path: PathBuf,
    temp_path: PathBuf,
}

struct DownloadTask<T> {
    info: DownloadInfo,
    tries: usize,
    task: Option<T>,
}

pub async fn dl_illust(
    client: &Client,
    illust_id: u64,
    mut save_path: PathBuf,
    directory_policy: DirectoryPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust_pages::get(client, illust_id).await?;

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

    let total_downloads = pages.len();
    let mut queued_downloads = Vec::with_capacity(total_downloads);

    // Create all DownloadInfos
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
        let mut temp_save_path = save_path.clone();
        save_path.push(filename);
        temp_save_path.push(format!("._{}", filename));

        queued_downloads.push(DownloadInfo {
            url,
            path: save_path,
            temp_path: temp_save_path,
        })
    }

    type OngoingDownload = Option<DownloadTask<JoinHandle<Result<(), anyhow::Error>>>>;
    let mut ongoing_downloads: [OngoingDownload; MAX_PARALLEL] = std::array::from_fn(|_| None); // https://stackoverflow.com/a/66776497 Don't like that I need to specify the type manually
    let mut finished_downloads = 0;

    'outer: loop {
        for slot in ongoing_downloads.iter_mut() {
            if let Some(d) = slot {
                // Check on ongoing download
                if let Some(task) = &mut d.task {
                    match task.await {
                        Ok(_) => {
                            // If went OK, clear slot
                            finished_downloads += 1;
                            *slot = None
                        }
                        Err(e) => {
                            if d.tries > MAX_RETRIES {
                                // Tried too many times, let it go.
                                eprintln!(
                                    "(Tried {} times) {}: {}",
                                    MAX_RETRIES,
                                    d.info.path.display(),
                                    e
                                );
                                finished_downloads += 1;
                                *slot = None
                            }
                            // If need to retry, keep current slot as is
                        }
                    }
                }
            }

            // If slot is empty (either first dl or previous dl finished), try to get next task
            if slot.is_none() {
                if let Some(info) = queued_downloads.pop() {
                    *slot = Some(DownloadTask {
                        info,
                        tries: 0,
                        task: None,
                    });
                }
            }

            // If slot is not None, spawn new task
            if let Some(d) = slot {
                let req = client
                    .get(&d.info.url)
                    .timeout(Duration::from_secs(TIMEOUT));
                let path_clone = d.info.path.clone();
                let temp_path_clone = d.info.temp_path.clone();

                d.tries += 1;
                d.task = Some(tokio::spawn(dl_image_to_disk(
                    path_clone,
                    temp_path_clone,
                    req,
                )));
            }

            // If all downloads have finished, exit loop
            if finished_downloads >= total_downloads {
                break 'outer;
            }
        }
    }

    Ok(())
}

async fn dl_image_to_disk(
    save_path: PathBuf,
    temp_save_path: PathBuf,
    req: RequestBuilder,
) -> Result<()> {
    let resp = req.send().await?;
    resp.error_for_status_ref()?;

    let mut file = File::create(&temp_save_path).await?;
    let mut stream = resp.bytes_stream();

    while let Some(data) = stream.next().await {
        file.write_all(&data?).await?
    }

    // Manual drop to properly close file
    drop(file);

    // Rename from temporary filename to permanent one
    rename(temp_save_path, save_path).await?;

    Ok(())
}
