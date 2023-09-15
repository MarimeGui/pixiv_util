use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::ApiError;

pub async fn get(client: &Client, illust_id: u64) -> Result<Vec<Page>, ApiError> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/illust/{}/pages?lang=en",
        illust_id
    ));
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
    pub body: Vec<Page>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub urls: Urls,
    pub width: usize,
    pub height: usize,
    /// 1 for Illust, 2 for Ugoira
    pub illust_type: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    pub thumb_mini: Option<String>,
    pub small: String,
    pub regular: String,
    pub original: String,
}
