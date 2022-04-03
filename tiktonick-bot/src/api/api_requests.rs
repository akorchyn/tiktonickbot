use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Deserialize;
use std::env;

pub(crate) struct ApiUrlGenerator {
    internal_api_url: String,
    internal_api_secret: String,
    api: String,
}

impl ApiUrlGenerator {
    pub(crate) fn from_env(api: String) -> Self {
        let internal_api_url = env::var("INTERNAL_API_URL").expect("INTERNAL_API_URL");
        let internal_api_secret = env::var("INTERNAL_API_SECRET").expect("INTERNAL_API_SECRET");
        ApiUrlGenerator {
            internal_api_url,
            internal_api_secret,
            api,
        }
    }

    pub(crate) fn get_content_by_id(&self, content_id: &str) -> String {
        format!(
            "{domain}/api/{api}/content_by_id/{id}",
            domain = self.internal_api_url,
            api = self.api,
            id = content_id
        )
    }

    pub(crate) fn get_user_content_by_type(
        &self,
        user_id: &str,
        content_type: &str,
        count: u8,
    ) -> String {
        format!(
            "{domain}/api/{api}/{user_id}/{content_type}/{count}",
            domain = self.internal_api_url,
            api = self.api,
            user_id = user_id,
            content_type = content_type,
            count = count
        )
    }

    pub(crate) fn get_user_info(&self, user_id: &str) -> String {
        format!(
            "{domain}/api/{api}/user_info/{user_id}",
            domain = self.internal_api_url,
            api = self.api,
            user_id = user_id
        )
    }

    pub(crate) async fn get_data<T>(&self, url: &str) -> anyhow::Result<Option<T>>
    where
        for<'a> T: Deserialize<'a>,
    {
        log::info!("Requesting data from {}", url);
        let client = Client::new();
        let response = client
            .get(url)
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
            Ok(None)
        } else {
            Err(anyhow::anyhow!(
                "Failed to get data from {}\nstatus is: {}\nresponse is:\n{}",
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
