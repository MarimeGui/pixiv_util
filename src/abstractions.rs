use anyhow::Result;

use crate::gen_http_client::SemaphoredClient;

const ILLUSTS_PER_PAGE: usize = 100; // Maximum allowed by API

pub async fn get_all_user_bookmarks<F>(
    client: SemaphoredClient,
    user_id: u64,
    mut f: F,
) -> Result<()>
where
    F: FnMut(u64) -> bool,
{
    // Request first page separately to get the number of bookmarks
    let first =
        crate::api_calls::user_bookmarks::get(client.clone(), user_id, 0, ILLUSTS_PER_PAGE).await?;
    for work in first.works {
        if work.is_masked {
            // Ignore illusts that have been removed
            continue;
        }
        if !f(work.id) {
            // Stop here if we have found an illust that we already have. This works because most recent bookmarks are received first
            return Ok(());
        }
    }

    // Iterate over every page
    let nb_pages =
        (first.total / ILLUSTS_PER_PAGE) + usize::from(first.total % ILLUSTS_PER_PAGE != 0);
    for page in 1..nb_pages {
        let body = crate::api_calls::user_bookmarks::get(
            client.clone(),
            user_id,
            page * ILLUSTS_PER_PAGE,
            ILLUSTS_PER_PAGE,
        )
        .await?;
        for work in body.works {
            if work.is_masked {
                continue;
            }
            if !f(work.id) {
                return Ok(());
            }
        }
    }

    Ok(())
}

// TODO: Shameless copy paste for now
pub async fn get_all_user_img_posts_with_tag<F>(
    client: SemaphoredClient,
    user_id: u64,
    tag: &str,
    mut f: F,
) -> Result<()>
where
    F: FnMut(u64) -> bool,
{
    // Request first page separately to get the number of bookmarks
    let first = crate::api_calls::user_illustmanga_tag::get(
        client.clone(),
        user_id,
        tag,
        0,
        ILLUSTS_PER_PAGE,
    )
    .await?;
    for work in first.works {
        if work.is_masked {
            // Ignore illusts that have been removed
            continue;
        }
        f(work.id);
    }

    // Iterate over every page
    let nb_pages =
        (first.total / ILLUSTS_PER_PAGE) + usize::from(first.total % ILLUSTS_PER_PAGE != 0);
    for page in 1..nb_pages {
        let body = crate::api_calls::user_illustmanga_tag::get(
            client.clone(),
            user_id,
            tag,
            page * ILLUSTS_PER_PAGE,
            ILLUSTS_PER_PAGE,
        )
        .await?;
        for work in body.works {
            if work.is_masked {
                continue;
            }
            f(work.id);
        }
    }

    Ok(())
}

pub async fn get_all_user_img_posts<F>(
    client: SemaphoredClient,
    user_id: u64,
    mut f: F,
) -> Result<()>
where
    F: FnMut(u64) -> bool,
{
    let user_info = crate::api_calls::user_info::get(client, user_id).await?;

    // Ignore bool indication as we already have all posts in one call with this API
    for illust_id in user_info.illusts {
        f(illust_id);
    }
    for illust_id in user_info.manga {
        f(illust_id);
    }

    Ok(())
}

pub async fn get_all_series_works<F>(
    client: SemaphoredClient,
    series_id: u64,
    mut f: F,
) -> Result<()>
where
    F: FnMut(u64) -> bool,
{
    let mut page_index = 1;
    let mut total = 0;

    loop {
        let body = crate::api_calls::series::get(client.clone(), series_id, page_index).await?;
        page_index += 1;

        total += body.page.series.len();

        for series in body.page.series {
            // TODO: series.order might be important
            // TODO: Need to check if works are most recent first in order to enable fast incremental
            f(series.work_id);
        }

        if total == body.page.total {
            break;
        }
    }

    Ok(())
}
