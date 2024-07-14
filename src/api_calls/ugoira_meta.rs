use serde::{Deserialize, Serialize};

use super::{ApiError, Root};
use crate::gen_http_client::SemaphoredClient;

pub async fn _get(client: SemaphoredClient, illust_id: u64) -> Result<Body, ApiError> {
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
