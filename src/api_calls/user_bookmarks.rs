use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{de_id, ApiError};

pub async fn get(client: &Client, user_id: u64, offset: u64, limit: u64) -> Result<Body, ApiError> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/user/{}/illusts/bookmarks?tag=&offset={}&limit={}&rest=show&lang=en",
        user_id, offset, limit,
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
    pub works: Vec<Work>,
    pub total: usize,
    // pub zone_config: ZoneConfig, // Seems to be mostly for ads
    pub extra_data: ExtraData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    #[serde(deserialize_with = "de_id")]
    pub id: u64,
    // pub title: String,
    // pub illust_type: i64,
    // pub x_restrict: i64,
    // pub restrict: i64,
    // pub sl: i64,
    // pub url: String,
    // pub description: String,
    // pub tags: Vec<String>,
    // pub user_id: String,
    // pub user_name: String,
    // pub width: i64,
    // pub height: i64,
    // pub page_count: usize,
    // pub is_bookmarkable: bool,
    // pub bookmark_data: Value,
    // pub alt: String,
    // pub title_caption_translation: TitleCaptionTranslation,
    // pub create_date: String,
    // pub update_date: String,
    // pub is_unlisted: bool,
    // pub is_masked: bool,
    // pub ai_type: i64,
    // pub profile_image_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleCaptionTranslation {
    pub work_title: Option<String>,
    pub work_caption: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraData {
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub title: String,
    pub description: String,
    pub canonical: String,
    pub ogp: Ogp,
    pub twitter: Twitter,
    pub alternate_languages: HashMap<String, String>,
    pub description_header: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ogp {
    pub description: String,
    pub image: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Twitter {
    pub description: String,
    pub image: String,
    pub title: String,
    pub card: String,
}
