use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{de_id, de_id_map, ApiError};

pub async fn get(client: &Client, user_id: u64) -> Result<Body, ApiError> {
    let req = client.get(format!(
        "https://www.pixiv.net/ajax/user/{}/profile/all",
        user_id,
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
    #[serde(deserialize_with = "de_id_map")]
    pub illusts: Vec<u64>,
    #[serde(deserialize_with = "de_id_map")]
    pub manga: Vec<u64>,
    #[serde(deserialize_with = "de_id_map")]
    pub novels: Vec<u64>,
    pub manga_series: Vec<MangaSeries>,
    pub novel_series: Vec<NovelSeries>,
    pub pickup: Vec<Pickup>,
    pub bookmark_count: BookmarkCount,
    pub external_site_works_status: ExternalSiteWorksStatus,
    pub request: Request,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaSeries {
    #[serde(deserialize_with = "de_id")]
    pub id: u64,
    #[serde(deserialize_with = "de_id")]
    pub user_id: u64,
    pub title: String,
    pub description: String,
    pub caption: String,
    pub total: usize,
    #[serde(rename = "content_order")]
    pub content_order: Value,
    pub url: String,
    // pub cover_image_sl: i64,
    #[serde(deserialize_with = "de_id")]
    pub first_illust_id: u64,
    #[serde(deserialize_with = "de_id")]
    pub latest_illust_id: u64,
    pub create_date: String,
    pub update_date: String,
    // pub watch_count: Value,
    pub is_watched: bool,
    pub is_notifying: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelSeries {
    #[serde(deserialize_with = "de_id")]
    pub id: u64,
    #[serde(deserialize_with = "de_id")]
    pub user_id: u64,
    pub user_name: String,
    pub profile_image_url: String,
    pub x_restrict: i64,
    pub is_original: bool,
    pub is_concluded: bool,
    pub genre_id: String, // TODO: Numerical value ?
    pub title: String,
    pub caption: String,
    pub language: String,
    pub tags: Vec<String>,
    pub published_content_count: i64,
    pub published_total_character_count: i64,
    pub published_total_word_count: i64,
    pub published_reading_time: i64,
    pub use_word_count: bool,
    pub last_published_content_timestamp: u64,
    pub created_timestamp: u64,
    pub updated_timestamp: u64,
    pub create_date: String,
    pub update_date: String,
    #[serde(deserialize_with = "de_id")]
    pub first_novel_id: u64,
    #[serde(deserialize_with = "de_id")]
    pub latest_novel_id: u64,
    pub display_series_content_count: i64,
    pub share_text: String,
    pub total: usize,
    pub first_episode: FirstEpisode,
    pub watch_count: Value,
    #[serde(rename = "maxXRestrict")]
    pub max_xrestrict: Value,
    pub cover: Cover,
    pub cover_setting_data: Value,
    pub is_watched: bool,
    pub is_notifying: bool,
    pub ai_type: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstEpisode {
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    pub urls: HashMap<String, String>,
}

/// Seems to be the things the user chose to put in front on their page, there are multiple types
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pickup {
    /// Known types: fanbox, illust, novel
    #[serde(rename = "type")]
    pub type_field: String,
    // #[serde(deserialize_with = "de_id")] // TODO: Anything else here is uncertain
    // pub id: u64,
    // pub title: String,
    // pub illust_type: Option<i64>,
    // pub x_restrict: i64,
    // pub restrict: i64,
    // pub sl: Option<i64>,
    // pub url: String,
    // pub description: String,
    // pub tags: Vec<String>,
    // pub user_id: String,
    // pub user_name: String,
    // pub width: Option<i64>,
    // pub height: Option<i64>,
    // pub page_count: Option<i64>,
    // pub is_bookmarkable: bool,
    // pub bookmark_data: Value,
    // pub alt: Option<String>,
    // pub title_caption_translation: TitleCaptionTranslation,
    // pub create_date: String,
    // pub update_date: String,
    // pub is_unlisted: bool,
    // pub is_masked: bool,
    // pub ai_type: i64,
    // pub urls: HashMap<String, String>,
    // pub deletable: bool,
    // pub draggable: bool,
    // pub content_url: String,
    // pub profile_image_url: Option<String>,
    // pub text_count: Option<i64>,
    // pub word_count: Option<i64>,
    // pub reading_time: Option<i64>,
    // pub use_word_count: Option<bool>,
    // pub bookmark_count: Option<i64>,
    // pub is_original: Option<bool>,
    // pub marker: Value,
    // pub series_id: Option<String>,
    // pub series_title: Option<String>,
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct TitleCaptionTranslation {
//     pub work_title: Value,
//     pub work_caption: Value,
// }

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkCount {
    pub public: Count,
    pub private: Count,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    pub illust: i64,
    pub novel: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSiteWorksStatus {
    pub booth: bool,
    pub sketch: bool,
    pub vroid_hub: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub show_request_tab: bool,
    pub show_request_sent_tab: bool,
    pub post_works: PostWorks,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostWorks {
    pub artworks: Vec<String>, // TODO: IDs
    pub novels: Vec<Value>,
}
