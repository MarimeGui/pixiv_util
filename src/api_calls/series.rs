use serde::{Deserialize, Serialize};

use super::{de_id, ApiError, Root};
use crate::gen_http_client::SemaphoredClient;

pub async fn get(client: SemaphoredClient, series_id: u64, page: usize) -> Result<Body, ApiError> {
    Root::query(
        client,
        &format!("https://www.pixiv.net/ajax/series/{}?p={}", series_id, page,),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub illust_series: Vec<SeriesInfo>,
    pub page: Page,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesInfo {
    #[serde(deserialize_with = "de_id")]
    pub id: u64,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub series: Vec<IllustPos>,
    pub total: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IllustPos {
    #[serde(deserialize_with = "de_id")]
    pub work_id: u64,
    pub order: usize,
}
