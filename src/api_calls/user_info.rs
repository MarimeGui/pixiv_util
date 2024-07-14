use serde::{Deserialize, Serialize};

use super::{de_id_map, ApiError, Root};
use crate::gen_http_client::SemaphoredClient;

pub async fn get(client: SemaphoredClient, user_id: u64) -> Result<Body, ApiError> {
    Root::query(
        client,
        &format!("https://www.pixiv.net/ajax/user/{}/profile/all", user_id,),
    )
    .await
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
}
