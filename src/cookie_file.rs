use std::path::PathBuf;

use anyhow::Result;
use dirs::config_dir;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

const COOKIE_FILE_NAME: &str = "pixiv_util_cookie";

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
    let mut file = File::create(get_cookie_file_path()?).await?;
    file.write_all(cookie.as_bytes()).await?;
    Ok(())
}
