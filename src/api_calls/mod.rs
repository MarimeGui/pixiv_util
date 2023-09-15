use reqwest::{Client, StatusCode};
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use thiserror::Error;

pub mod illust;
pub mod illust_pages;
pub mod novel;
pub mod series;
pub mod ugoira_meta;
pub mod user_bookmarks;
pub mod user_info;

// -----

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root<T> {
    pub error: bool,
    pub message: String,
    pub body: T,
}

impl<T: Serialize + DeserializeOwned> Root<T> {
    pub async fn query(client: &Client, url: &str) -> Result<T, ApiError> {
        let req = client.get(url);
        let resp = req.send().await.map_err(ApiError::Network)?;
        let status_code = resp.status();

        let root: Root<T> = resp.json().await.map_err(ApiError::Parse)?;

        if root.error {
            return Err(ApiError::Application {
                message: root.message,
                status_code,
            });
        }

        Ok(root.body)
    }
}

// -----

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("problem with http/network")]
    Network(#[source] reqwest::Error),
    #[error("couldn't parse received json")]
    Parse(#[source] reqwest::Error),
    #[error("\"{message}\" ({status_code})")]
    Application {
        message: String,
        status_code: StatusCode,
    },
}

// -----

// https://www.reddit.com/r/rust/comments/fcz4yb/how_do_you_deserialize_strings_integers_to_float/
fn de_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num.as_u64().ok_or(de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}

fn de_id_map<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u64>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Array(_) => {
            // TODO: Does the server ever reply something non-empty with arrays ?
            vec![]
        }
        Value::Object(o) => {
            let mut out = Vec::with_capacity(o.len());
            for k in o.keys() {
                out.push(k.parse().map_err(de::Error::custom)?)
            }
            out
        }
        _ => return Err(de::Error::custom("wrong type")),
    })
}
