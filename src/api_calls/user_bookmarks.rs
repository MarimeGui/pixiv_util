use serde::{Deserialize, Serialize};

use super::{de_id, ApiError, Root};
use crate::gen_http_client::SemaphoredClient;

pub async fn get(
    client: SemaphoredClient,
    user_id: u64,
    offset: usize,
    limit: usize,
    visibility: Visibility,
) -> Result<Body, ApiError> {
    Root::query(
        client,
        &format!(
            "https://www.pixiv.net/ajax/user/{}/illusts/bookmarks?tag=&offset={}&limit={}&rest={}",
            user_id,
            offset,
            limit,
            visibility.to_rest()
        ),
    )
    .await
}

#[derive(Copy, Clone)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    const fn to_rest(self) -> &'static str {
        match self {
            Visibility::Public => "show",
            Visibility::Private => "hide",
        }
    }
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
