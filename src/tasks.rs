use std::{collections::BTreeMap, path::PathBuf};

use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

pub async fn dl_series(client: &Client, series_id: u64) -> Result<()> {
    let mut page_index: u64 = 1;
    let mut illusts = BTreeMap::new();

    loop {
        let body = crate::api_calls::series::get(client, series_id, page_index).await?;
        page_index += 1;

        // Add all illusts
        for series in body.page.series {
            illusts.insert(series.order, series.work_id);
        }

        // If we have all images then stop
        if illusts.len() == body.page.total {
            break;
        }
    }

    let illusts: Vec<&str> = illusts.values().map(|s| s.as_str()).collect();

    let mut tasks = Vec::new();
    for illust in illusts {
        let client = client.clone();
        let illust_id = illust.parse()?;
        tasks.push(tokio::spawn(
            async move { dl_illust(&client, illust_id).await },
        ));
    }

    for task in tasks {
        task.await??
    }

    Ok(())
}

// TODO: When Downloading series, being able to choose between putting all files in the same folder or have everything in a separate folder

pub async fn dl_illust(client: &Client, illust_id: u64) -> Result<()> {
    let pages = crate::api_calls::illust::get(client, illust_id).await?;

    let in_folder = pages.len() > 1;
    if in_folder {
        create_dir_all(illust_id.to_string()).await?;
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

        let mut save_path = PathBuf::new();

        // If multiple pages, put everything in folder
        if in_folder {
            save_path.push(illust_id.to_string());
        }

        save_path.push(filename);

        // Make the query
        let req = client.get(url);

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
