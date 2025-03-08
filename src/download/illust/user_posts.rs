use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    gen_http_client::SemaphoredClient, update_file::create_update_file, DownloadIllustModes,
};

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

pub async fn illusts_from_user_posts(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    make_update_file: bool,
    user_id: u64,
    illust_tx: UnboundedSender<u64>,
) -> Result<()> {
    let user_info = crate::api_calls::user_info::get(client.clone(), user_id).await?;

    for illust_id in user_info.illusts.iter().chain(user_info.manga.iter()) {
        illust_tx.send(*illust_id)?;
    }

    if make_update_file {
        create_update_file(
            &dest_dir,
            &DownloadIllustModes::UserPosts { tag: None, user_id },
        )?;
    }

    Ok(())
}

pub async fn illusts_from_user_posts_with_tag(
    client: SemaphoredClient,
    dest_dir: PathBuf,
    make_update_file: bool,
    user_id: u64,
    tag: &str,
    illust_tx: UnboundedSender<u64>,
) -> Result<()> {
    let mut processed = 0;

    loop {
        let body = crate::api_calls::user_illustmanga_tag::get(
            client.clone(),
            user_id,
            tag,
            processed,
            ILLUSTS_PER_PAGE,
        )
        .await?;

        for work in &body.works {
            illust_tx.send(work.id)?;
        }

        processed += body.works.len();

        // Got every illust
        if processed >= body.total {
            break;
        }
    }

    if make_update_file {
        create_update_file(
            &dest_dir,
            &DownloadIllustModes::UserPosts {
                tag: Some(tag.to_owned()),
                user_id,
            },
        )?;
    }

    Ok(())
}
