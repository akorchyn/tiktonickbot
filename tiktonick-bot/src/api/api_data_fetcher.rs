use crate::api::Api;
use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Deserialize;
use std::env;

pub(crate) type Username<'a> = &'a str;
pub(crate) type ContentType<'a> = &'a str;

pub(crate) enum Request<'a> {
    UserData(Username<'a>),
    Content(Username<'a>, ContentType<'a>, u8),
    ContentById(&'a str),
}

impl<'a> Request<'a> {
    pub(crate) fn request_string(&self) -> String {
        match self {
            Request::UserData(username) => format!("user_info/{}", username),
            Request::Content(username, content_type, count) => {
                format!(
                    "{content_type}/{username}/{count}",
                    username = username,
                    content_type = content_type,
                    count = count
                )
            }
            Request::ContentById(id) => format!("content_by_id/{}", id),
        }
    }
}

impl Api {
    fn api_to_string(&self) -> String {
        match self {
            #[cfg(feature = "twitter")]
            Api::Twitter => "twitter",
            #[cfg(feature = "tiktok")]
            Api::Tiktok => "tiktok",
            #[cfg(feature = "instagram")]
            Api::Instagram => "instagram",
        }
        .to_string()
    }
}

pub(crate) struct ApiDataFetcher {
    internal_api_url: String,
    internal_api_secret: String,
    api: String,
}

impl ApiDataFetcher {
    pub(crate) fn from_env(api: Api) -> Self {
        let internal_api_url = env::var("INTERNAL_API_URL").expect("INTERNAL_API_URL");
        let internal_api_secret = env::var("INTERNAL_API_SECRET").expect("INTERNAL_API_SECRET");

        ApiDataFetcher {
            internal_api_url,
            internal_api_secret,
            api: api.api_to_string(),
        }
    }

    pub(crate) async fn get_data<'a, T>(&self, request: Request<'a>) -> anyhow::Result<Option<T>>
    where
        for<'b> T: Deserialize<'b>,
    {
        let url = format!(
            "{url}/api/{api_type}/{request}",
            url = self.internal_api_url,
            api_type = self.api,
            request = request.request_string()
        );
        log::info!("Requesting data from {}", &url);
        let client = Client::new();
        let response = client
            .get(&url)
            .bearer_auth(&self.internal_api_secret)
            .send()
            .await?;
        log::info!("Response status: {}", response.status());
        if response.status().is_success() {
            let text = response.text().await?;
            let data = serde_json::from_str::<T>(&text);
            if let Ok(data) = data {
                log::info!("Data received");
                Ok(Some(data))
            } else {
                Err(anyhow::anyhow!(
                    "Failed to decode json response: {}\ndata is:\n{}",
                    data.err().unwrap(),
                    text
                ))
            }
        } else if response.status() == 404 {
            // User/content not found
            Ok(None)
        } else if response.status() == 503 {
            // Service unavailable (Proxy error)
            Err(anyhow::anyhow!("Proxy error fetching: {}\n", url,))
        } else {
            Err(anyhow::anyhow!(
                "Failed to fetch data from: {}\nstatus: {}\nresponse: {}",
                url,
                response.status(),
                response.text().await?
            ))
        }
    }
}

pub(crate) async fn get_full_link(short_url: &str) -> anyhow::Result<String> {
    let client = reqwest::ClientBuilder::new()
        .redirect(Policy::limited(2))
        .build()?;
    match client
        .head(short_url)
        .header("User-Agent", "curl/7.22.0 (x86_64-pc-linux-gnu)") // For some reason reqwest hangs without it
        .send()
        .await
    {
        Ok(result) => Ok(result.url().to_string()),
        Err(e) => {
            log::warn!("Failed to send head request. {}", e);
            Err(anyhow::anyhow!("Error: {}", e))
        }
    }
}
