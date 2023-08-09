use anyhow::{anyhow, Result};

pub fn parse_illust_id(s: &str) -> Result<u64> {
    // Is a straight id
    if let Ok(v) = s.parse() {
        return Ok(v);
    }

    // Is a URL
    if let Some(part) = s.split_once("artworks/") {
        let id_s = match part.1.split_once("#") {
            Some((s, _)) => s,
            None => part.1,
        };
        if let Ok(v) = id_s.parse() {
            return Ok(v);
        }
    }

    return Err(anyhow!("cannot recognize illust id"));
}
