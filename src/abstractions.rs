use anyhow::Result;
use reqwest::Client;

pub async fn get_all_user_bookmarks<F>(client: &Client, user_id: u64, mut f: F) -> Result<()>
where
    F: FnMut(u64),
{
    let mut page = 0;

    loop {
        let body = crate::api_calls::user_bookmarks::get(client, user_id, page * 100, 100).await?;
        page += 1;
        for work in &body.works {
            f(work.id)
        }
        // If answer contained less than a 100 works, we've reached the end
        if body.works.len() < 100 {
            break;
        }
    }

    Ok(())
}

pub async fn get_all_series_works<F>(client: &Client, series_id: u64, mut f: F) -> Result<()>
where
    F: FnMut(u64),
{
    let mut page_index: u64 = 1;
    let mut total = 0;

    loop {
        let body = crate::api_calls::series::get(client, series_id, page_index).await?;
        page_index += 1;

        total += body.page.series.len();

        for series in body.page.series {
            // TODO: series.order might be important
            f(series.work_id)
        }

        if total == body.page.total {
            break;
        }
    }

    Ok(())
}
