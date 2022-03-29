use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Deserialize;
use std::env;

// Twitter defaults
const TWITTER_TWEET_FIELDS: &'static str =
    "id,text,attachments,author_id,in_reply_to_user_id,referenced_tweets,source";
const TWITTER_EXPANSIONS: &'static str = "author_id,attachments.media_keys";
const TWITTER_MEDIA_FIELDS: &'static str = "preview_image_url,url,media_key";

pub(crate) struct ApiUrlGenerator {
    internal_api_url: String,
    twitter_url: String,
}

impl ApiUrlGenerator {
    pub(crate) fn from_env() -> Self {
        let internal_api_url = env::var("INTERNAL_API_URL").expect("INTERNAL_API_URL");
        let twitter_url = env::var("TWITTER_API_URL").expect("TWITTER_API_URL");
        ApiUrlGenerator {
            internal_api_url,
            twitter_url,
        }
    }

    pub(crate) fn get_tiktok_video_by_id_link(&self, video_id: &str) -> String {
        format!(
            "{}/api/video_by_id/?video_id={}",
            self.internal_api_url, video_id
        )
    }

    pub(crate) fn get_tiktok_api_call(&self, api: &str, username: &str, count: u8) -> String {
        format!(
            "{}/api/{}/?username={}&count={}",
            self.internal_api_url, api, username, count
        )
    }

    pub(crate) fn get_tiktok_user_info(&self, username: &str) -> String {
        format!(
            "{}/api/user_info/?username={}",
            self.internal_api_url, username
        )
    }

    pub(crate) fn get_tiktok_status(&self) -> String {
        format!("{}/api/status", self.internal_api_url)
    }

    pub(crate) fn get_twitter_api_call(&self, api: &str, username: &str, count: u8) -> String {
        format!(
            "{}/2/users/{}/{}?tweet.fields={TWITTER_TWEET_FIELDS}&max_results={MAX_RESULT}&expansions={TWITTER_EXPANSIONS}&media.fields={TWITTER_MEDIA_FIELDS}",
            self.twitter_url, username, api, MAX_RESULT=count, TWITTER_TWEET_FIELDS=TWITTER_TWEET_FIELDS, TWITTER_EXPANSIONS=TWITTER_EXPANSIONS, TWITTER_MEDIA_FIELDS=TWITTER_MEDIA_FIELDS
        )
    }

    pub(crate) fn get_twitter_user_info(&self, username: &str) -> String {
        format!("{}/2/users/by/username/{}", self.twitter_url, username)
    }

    pub(crate) fn get_tweet_link(&self, tweet_id: &str) -> String {
        format!(
            "{}/2/tweets/{}?tweet.fields={}&expansions={}&media.fields={}",
            self.twitter_url,
            tweet_id,
            TWITTER_TWEET_FIELDS,
            TWITTER_EXPANSIONS,
            TWITTER_MEDIA_FIELDS
        )
    }

    pub(crate) fn get_change_proxy_link(&self) -> String {
        format!("{}/api/change_proxy", self.internal_api_url)
    }
}

pub(crate) async fn get_data<T>(url: &str, secret: &str) -> anyhow::Result<Option<T>>
where
    for<'a> T: Deserialize<'a>,
{
    log::info!("Requesting data from {}", url);
    let client = Client::new();
    let response = client.get(url).bearer_auth(secret).send().await?;
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
