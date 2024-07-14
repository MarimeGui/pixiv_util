use serde::{Deserialize, Serialize};

use crate::gen_http_client::SemaphoredClient;

use super::{ApiError, Root};

pub async fn get(client: SemaphoredClient, illust_id: u64) -> Result<Vec<Page>, ApiError> {
    Root::query(
        client,
        &format!("https://www.pixiv.net/ajax/illust/{}/pages", illust_id),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub urls: Urls,
    pub width: usize,
    pub height: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    pub thumb_mini: Option<String>,
    pub small: String,
    pub regular: String,
    pub original: String,
}
