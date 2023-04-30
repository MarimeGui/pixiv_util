use reqwest::{
    header::{HeaderMap, HeaderValue, InvalidHeaderValue, COOKIE, REFERER, USER_AGENT},
    Client, ClientBuilder,
};

// Clippy is not happy with this, saying I should use a static with lazy_static in this case...
const MY_USER_AGENT: HeaderValue = HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36");
const MY_REFERER: HeaderValue = HeaderValue::from_static("https://www.pixiv.net/");

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

pub fn make_client(headers: HeaderMap) -> reqwest::Result<Client> {
    ClientBuilder::new().default_headers(headers).build()
}
