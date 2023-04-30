use std::path::PathBuf;

use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

use crate::FolderPolicy;

// -------

pub async fn dl_illust(
    client: &Client,
    illust_id: u64,
    output_folder: Option<PathBuf>,
    folder_policy: FolderPolicy,
) -> Result<()> {
    let pages = crate::api_calls::illust::get(client, illust_id).await?;

    let in_folder = match folder_policy {
        FolderPolicy::AlwaysCreate => true,
        FolderPolicy::NeverCreate => false,
        FolderPolicy::Auto => pages.len() > 1,
    };

    // Use path if provided, otherwise use current folder
    let mut save_path = if let Some(o) = output_folder {
        o
    } else {
        PathBuf::new()
    };

    // If multiple pages, put everything in folder
    if in_folder {
        create_dir_all(illust_id.to_string()).await?;
        save_path.push(illust_id.to_string());
    }

    let mut downloads = Vec::new();
    for page in pages.iter() {
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
    let data = resp.bytes().await?;
    let mut file = File::create(save_path).await?;
    file.write_all(&data).await?;
    Ok(())
}
