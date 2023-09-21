use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ApiError, Root};

pub async fn _get(client: &Client, illust_id: u64) -> Result<Body, ApiError> {
    Root::query(
        client,
        &format!(
            "https://www.pixiv.net/ajax/illust/{}/ugoira_meta",
            illust_id,
        ),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    // src
    pub original_src: String,
    pub mime_type: String,
    pub frames: Vec<Frame>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub file: String,
    pub delay: u64,
}
