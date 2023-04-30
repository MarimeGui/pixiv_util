use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::de_id;

pub async fn get(client: &Client, series_id: u64, page: u64) -> Result<Body> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/series/{}?p={}&lang=en",
        series_id, page,
    ));
    let resp = req.send().await?;
    let status_code = resp.status();

    let root = resp.json::<Root>().await?;

    if root.error {
        return Err(anyhow::anyhow!(
            "Server returned: \"{}\" ({})",
            root.message,
            status_code
        ));
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
    // pub tag_translation: HashMap<String, HashMap<String, String>>, // TODO: Leave this field out for now as response switches between an array and a map
    pub thumbnails: Thumbnails,
    pub illust_series: Vec<RelatedSeries>,
    pub requests: Vec<Value>,
    pub users: Vec<User>,
    pub page: Page,
    pub extra_data: ExtraData,
    pub zone_config: ZoneConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnails {
    pub illust: Vec<Illust>,
    pub novel: Vec<Value>,
    pub novel_series: Vec<Value>,
    pub novel_draft: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Illust {
    pub id: String,
    pub title: String,
    pub illust_type: i64,
    pub x_restrict: i64,
    pub restrict: i64,
    pub sl: i64,
    pub url: String,
    pub description: String,
    pub tags: Vec<String>,
    pub user_id: String,
    pub user_name: String,
    pub width: i64,
    pub height: i64,
    pub page_count: i64,
    pub is_bookmarkable: bool,
    pub bookmark_data: Value,
    pub alt: String,
    pub title_caption_translation: TitleCaptionTranslation,
    pub create_date: String,
    pub update_date: String,
    pub is_unlisted: bool,
    pub is_masked: bool,
    pub ai_type: i64,
    pub urls: HashMap<String, String>,
    pub series_id: String,
    pub series_title: String,
    pub profile_image_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleCaptionTranslation {
    pub work_title: Value,
    pub work_caption: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedSeries {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub caption: String,
    pub total: usize,
    #[serde(rename = "content_order")]
    pub content_order: Value,
    pub url: String,
    pub cover_image_sl: i64,
    pub first_illust_id: String,
    pub latest_illust_id: String,
    pub create_date: String,
    pub update_date: String,
    pub watch_count: Value,
    pub is_watched: bool,
    pub is_notifying: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub partial: i64,
    pub comment: String,
    pub followed_back: bool,
    pub user_id: String,
    pub name: String,
    pub image: String,
    pub image_big: String,
    pub premium: bool,
    pub is_followed: bool,
    pub is_mypixiv: bool,
    pub is_blocking: bool,
    pub background: Value,
    pub accept_request: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub series: Vec<IllustPos>,
    pub is_set_cover: bool,
    pub series_id: u64,
    pub other_series_id: String,
    pub recent_updated_work_ids: Vec<u64>,
    pub total: usize,
    pub is_watched: bool,
    pub is_notifying: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IllustPos {
    #[serde(deserialize_with = "de_id")]
    pub work_id: u64,
    pub order: usize,
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
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ogp {
    #[serde(rename = "type")]
    pub type_field: String,
    pub title: String,
    pub description: String,
    pub image: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Twitter {
    pub card: String,
    pub site: String,
    pub title: String,
    pub description: String,
    pub image: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneConfig {
    pub header: Header,
    pub footer: Footer,
    pub responsive: Responsive,
    pub rectangle: Rectangle,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Footer {
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Responsive {
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rectangle {
    pub url: String,
}
