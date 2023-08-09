use anyhow::{anyhow, Result};

pub fn sanitize_cookie(cookie: &str) -> Result<String> {
    // TODO: Remove useless fields

    // Strip header field name if present
    let stripped = if let Some(s) = cookie.strip_prefix("Cookie: ") {
        s
    } else {
        cookie
    };

    // Check that there are at least 5 fields
    if stripped.split("; ").count() > 5 {
        Ok(stripped.to_string())
    } else {
        Err(anyhow!("provided string does not look like a valid cookie"))
    }
}

pub fn parse_illust_id(s: &str) -> Result<u64> {
    // Is a straight id
    if let Ok(v) = s.parse() {
        return Ok(v);
    }

    // Is a URL like https://www.pixiv.net/en/artworks/{illust_id}#*
    if let Some(part) = s.split_once("artworks/") {
        let id_s = match part.1.split_once('#') {
            Some((s, _)) => s,
            None => part.1,
        };
        if let Ok(v) = id_s.parse() {
            return Ok(v);
        }
    }

    Err(anyhow!("cannot recognize illust id"))
}

pub fn parse_series_id(s: &str) -> Result<u64> {
    // Is a straight id
    if let Ok(v) = s.parse() {
        return Ok(v);
    }

    // Is a URL like https://www.pixiv.net/user/*/series/{series_id}
    if let Some(part) = s.split_once("series/") {
        if let Ok(v) = part.1.parse() {
            return Ok(v);
        }
    }

    Err(anyhow!("cannot recognize series id"))
}

pub fn parse_user_id(s: &str) -> Result<u64> {
    // Is a straight id
    if let Ok(v) = s.parse() {
        return Ok(v);
    }

    // Is a URL like https://www.pixiv.net/en/users/{user_id}/*
    if let Some(part) = s.split_once("users/") {
        let id_s = match part.1.split_once('/') {
            Some((s, _)) => s,
            None => part.1,
        };
        if let Ok(v) = id_s.parse() {
            return Ok(v);
        }
    }

    Err(anyhow!("cannot recognize user id"))
}
