use std::path::PathBuf;

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

    // If incremental is active, list all files
    let file_list = if let Some(p) = params.incremental {
        Some(list_all_files(p)?)
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
                return;
            }
        }
        let client = client.clone();
        let output_dir = params.output_directory.clone();
        tasks.push(tokio::spawn(async move {
            dl_illust(&client, illust_id, output_dir, params.directory_policy).await
        }));
    };

    // Run all tasks
    match params.mode {
        DownloadModesSubcommands::Individual { illust_ids } => {
            for illust_id in illust_ids {
                f(illust_id)
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

pub async fn dl_illust(
    client: &Client,
    illust_id: u64,
    output_directory: Option<PathBuf>,
    directory_policy: DirectoryPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust::get(client, illust_id).await?;

    let in_dir = match directory_policy {
        DirectoryPolicy::AlwaysCreate => true,
        DirectoryPolicy::NeverCreate => false,
        DirectoryPolicy::CreateIfMultiple => pages.len() > 1,
    };

    // Use path if provided, otherwise use current dir
    let mut save_path = if let Some(o) = output_directory {
        o
    } else {
        PathBuf::new()
    };

    // If multiple pages, put everything in dir
    if in_dir {
        create_dir_all(illust_id.to_string()).await?;
        save_path.push(illust_id.to_string());
    }

    let mut downloads = Vec::new();
    for page in pages.iter() {
        // TODO: Move filename extraction to future ? Might make it a tiny bit faster
        // Get the URL for this image
        let url = &page.urls.original;

        // Extract filename
        let filename = {
            match url.rsplit('/').next() {
                Some(p) => p,
                None => url,
            }
        };

        // Append filename
        let mut save_path = save_path.clone();
        save_path.push(filename);

        // Make the query
        let req = client.get(url);

        // Perform download
        downloads.push(tokio::spawn(dl_image_to_disk(save_path, req)));
    }

    // Check if everything went okay
    for task in downloads {
        task.await??
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
