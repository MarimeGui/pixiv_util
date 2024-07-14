use std::sync::Arc;

use reqwest::{
    header::{HeaderMap, HeaderValue, InvalidHeaderValue, COOKIE, REFERER, USER_AGENT},
    Client, ClientBuilder,
};
use tokio::sync::Semaphore;

#[allow(clippy::declare_interior_mutable_const)]
const MY_USER_AGENT: HeaderValue = HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36");
#[allow(clippy::declare_interior_mutable_const)]
const MY_REFERER: HeaderValue = HeaderValue::from_static("https://www.pixiv.net/");

/// Everywhere in this program, max HTTP requests that can run at once
const MAX_CONCURRENT_REQUESTS: usize = 50;

pub fn make_headers(
    user_cookie: Option<&str>,
) -> std::result::Result<HeaderMap, InvalidHeaderValue> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, MY_USER_AGENT);
    headers.insert(REFERER, MY_REFERER);
    if let Some(c) = user_cookie {
        headers.insert(COOKIE, HeaderValue::from_str(c)?);
    }

    Ok(headers)
}

pub fn make_client(headers: HeaderMap) -> reqwest::Result<SemaphoredClient> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    let client = ClientBuilder::new()
        .default_headers(headers)
        .gzip(true)
        .build()?;

    Ok(SemaphoredClient { semaphore, client })
}

// https://users.rust-lang.org/t/reqwest-http-client-fails-when-too-much-concurrency/55644/2

/// Used for limiting concurrency of requests, i.e., not having 1000s of requests at once. Acquire permit from semaphore before using client, then drop permit when done
#[derive(Clone)]
pub struct SemaphoredClient {
    pub semaphore: Arc<Semaphore>,
    pub client: Client,
}
