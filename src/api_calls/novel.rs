use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{ApiError, Root};
use crate::gen_http_client::SemaphoredClient;

pub async fn get(client: SemaphoredClient, novel_id: u64) -> Result<NovelInfo, ApiError> {
    Root::query(
        client,
        &format!("https://www.pixiv.net/ajax/novel/{}", novel_id),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelInfo {
    pub create_date: String,
    pub upload_date: String,
    pub description: String,
    pub title: String,
    pub content: String,
    pub text_embedded_images: Option<HashMap<u64, EmbeddedImage>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedImage {
    pub novel_image_id: String,
    pub sl: String,
    pub urls: HashMap<String, String>,
}
