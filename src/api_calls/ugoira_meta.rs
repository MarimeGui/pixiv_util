use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::ApiError;

pub async fn get(client: &Client, illust_id: u64) -> Result<Body, ApiError> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/illust/{}/ugoira_meta?lang=en",
        illust_id,
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
    pub body: Body,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    // src
    pub original_src: String,
    // mime_type
    pub frames: Vec<Frame>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub file: String,
    pub delay: u64,
}
