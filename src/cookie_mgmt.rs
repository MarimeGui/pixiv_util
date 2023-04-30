use std::path::PathBuf;

use anyhow::Result;
use dirs::config_dir;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

const COOKIE_FILE_NAME: &str = "pixiv_util_cookie";

// TODO: Automatically update cookie with server answers ?

fn sanitize(cookie: &str) -> &str {
    // TODO: Remove useless fields
    if let Some(s) = cookie.strip_prefix("Cookie: ") {
        s
    } else {
        cookie
    }
}

pub fn get_cookie_file_path() -> Result<PathBuf> {
    let mut config = match config_dir() {
        Some(c) => c,
        _ => return Err(anyhow::anyhow!("No suitable configuration folder !")),
    };

    config.push(COOKIE_FILE_NAME);

    Ok(config)
}

pub async fn get_cookie_from_file() -> Result<String> {
    let mut file = File::open(get_cookie_file_path()?).await?;
    let mut s = String::new();
    file.read_to_string(&mut s).await?;
    Ok(s)
}

pub async fn set_cookie_to_file(cookie: &str) -> Result<()> {
    let cookie = sanitize(cookie);
    let mut file = File::create(get_cookie_file_path()?).await?;
    file.write_all(cookie.as_bytes()).await?;
    Ok(())
}

pub async fn retrieve_cookie(cookie_override: Option<String>) -> Option<String> {
    match cookie_override {
        // TODO: Doing String -> &str -> String... Especially since we're only using it as a &str later
        Some(overridden) => Some(sanitize(overridden.as_str()).to_string()),
        // TODO: This should be a bit smarter, like if the file is empty
        None => match get_cookie_from_file().await {
            Ok(c) => Some(c),
            Err(_) => None,
        },
    }
}
