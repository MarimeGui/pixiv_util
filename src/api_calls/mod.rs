use serde::{de, Deserialize, Deserializer};
use serde_json::Value;

pub mod illust;
pub mod series;
pub mod user_bookmarks;

// https://transform.tools/json-to-rust-serde
// Best website ever

// TODO: Macro for get fns ?

// https://www.reddit.com/r/rust/comments/fcz4yb/how_do_you_deserialize_strings_integers_to_float/
fn de_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num.as_u64().ok_or(de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}
