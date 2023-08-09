use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::ApiError;

pub async fn get(client: &Client, novel_id: u64) -> Result<NovelInfo, ApiError> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/novel/{}?lang=en",
        novel_id
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
    pub body: NovelInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelInfo {
    pub content: String,
    pub create_date: String,
    pub description: String,
    pub text_embedded_images: HashMap<u64, EmbeddedImage>,
    pub title: String,
    pub upload_date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedImage {
    pub novel_image_id: String,
    pub sl: String,
    pub urls: HashMap<String, String>,
}
