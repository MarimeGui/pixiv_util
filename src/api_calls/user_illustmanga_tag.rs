use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{de_id, ApiError, Root};

pub async fn _get(
    client: &Client,
    tag: String,
    offset: usize,
    limit: usize,
) -> Result<Body, ApiError> {
    Root::query(
        client,
        &format!(
            "https://www.pixiv.net/ajax/user/817796/illustmanga/tag?tag={}&offset={}&limit={}",
            tag, offset, limit,
        ),
    )
    .await
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub works: Vec<Work>,
    pub total: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    #[serde(deserialize_with = "de_id")]
    pub id: u64,
    pub is_masked: bool,
}
