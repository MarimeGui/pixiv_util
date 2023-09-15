use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{de_id, ApiError, Root};

pub async fn get(client: &Client, illust_id: u64) -> Result<IllustInfo, ApiError> {
    Root::query(
        client,
        &format!("https://www.pixiv.net/ajax/illust/{}", illust_id),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IllustInfo {
    pub illust_title: String,
    /// 1 for Illust, 2 for Ugoira
    pub illust_type: usize,
    pub create_date: String,
    pub upload_date: String,
    #[serde(deserialize_with = "de_id")]
    pub user_id: u64,
    pub page_count: usize,
}
