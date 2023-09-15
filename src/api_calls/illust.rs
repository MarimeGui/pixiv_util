use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{de_id, ApiError};

pub async fn get(client: &Client, illust_id: u64) -> Result<IllustInfo, ApiError> {
    let req = client.get(format!("https://www.pixiv.net/ajax/illust/{}", illust_id));
    let resp = req.send().await.map_err(ApiError::Network)?;
    let status_code = resp.status();

    let root = resp.json::<Root>().await.map_err(ApiError::Parse)?;

    if root.error {
        return Err(ApiError::Application {
            message: root.message,
            status_code,
        });
    }

    Ok(root.body)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub error: bool,
    pub message: String,
    pub body: IllustInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IllustInfo {
    pub create_date: String,
    pub upload_date: String,
    #[serde(deserialize_with = "de_id")]
    pub user_id: u64,
    /// 1 for Illust, 2 for Ugoira
    pub illust_type: usize,
    pub page_count: usize,
}
