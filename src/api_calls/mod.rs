use reqwest::{Client, StatusCode};
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{from_slice, from_value, Value};
use thiserror::Error;

pub mod illust;
pub mod illust_pages;
pub mod novel;
pub mod series;
pub mod ugoira_meta;
pub mod user_bookmarks;
pub mod user_illustmanga_tag;
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
        let full = resp.bytes().await.map_err(ApiError::Network)?;

        // Check for empty response
        if full.is_empty() {
            return Err(ApiError::EmptyResponse { status_code });
        }

        // Parse root first
        let root: Root<Value> = from_slice(&full).map_err(ApiError::JSONParse)?;

        // Check for application error
        if root.error {
            return Err(ApiError::ServerApplication {
                message: root.message,
                status_code,
            });
        }

        // Check for return code
        if status_code != StatusCode::OK {
            return Err(ApiError::ServerHTTP { status_code });
        }

        // Parse body next
        let body: T = from_value(root.body).map_err(ApiError::JSONParse)?;

        Ok(body)
    }
}

// -----

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("problem with http/network")]
    Network(#[source] reqwest::Error),
    #[error("server returned an empty response with code {status_code}")]
    EmptyResponse { status_code: StatusCode },
    #[error("couldn't parse received json")]
    JSONParse(#[source] serde_json::Error),
    #[error("server returned \"{message}\" ({status_code})")]
    ServerApplication {
        message: String,
        status_code: StatusCode,
    },
    #[error("server returned {status_code}")]
    ServerHTTP { status_code: StatusCode },
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
