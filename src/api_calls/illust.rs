use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub async fn get(client: &Client, illust_id: u64) -> Result<Vec<Body>> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/illust/{}/pages?lang=en",
        illust_id
    ));
    let resp = req.send().await?;
    let status_code = resp.status();

    let pages = resp.json::<Root>().await?;

    if pages.error {
        return Err(anyhow::anyhow!(
            "Server returned: \"{}\" ({})",
            pages.message,
            status_code
        ));
    }

    Ok(pages.body)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub error: bool,
    pub message: String,
    pub body: Vec<Body>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub urls: Urls, // TODO: Check if this varies with the size of the image
    pub width: usize,
    pub height: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    #[serde(rename = "thumb_mini")]
    pub thumb_mini: String,
    pub small: String,
    pub regular: String,
    pub original: String,
}
