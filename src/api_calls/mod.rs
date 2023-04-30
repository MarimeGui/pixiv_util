use serde::{de, Deserialize, Deserializer};
use serde_json::Value;

pub mod illust;
pub mod series;
pub mod user_bookmarks;
pub mod user_info;

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
