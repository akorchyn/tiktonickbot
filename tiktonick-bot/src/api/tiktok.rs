use reqwest;

use std::env;

use crate::api::{
    Api, ApiAlive, ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload, DataType,
    DatabaseInfoProvider, GenerateMessage, GetId, OutputType, ReturnDataForDownload,
    ReturnTextInfo, ReturnUserInfo, ReturnUsername, SubscriptionType,
};
use crate::regexp;

use anyhow;
use async_trait::async_trait;
use html_escape;
use reqwest::redirect::Policy;
use serde::{self, Deserialize};
use serde_json;
use teloxide::types::ParseMode::Html;

#[derive(Deserialize, Debug)]
pub(crate) struct UserInfo {
    #[serde(rename = "uniqueId")]
    pub(crate) unique_user_id: String,
    #[serde(rename = "nickname")]
    pub(crate) nickname: String,
}

impl ReturnUserInfo for UserInfo {
    fn id(&self) -> &str {
        &self.unique_user_id
    }
    fn unique_user_name(&self) -> &str {
        &self.unique_user_id
    }
    fn nickname(&self) -> &str {
        &self.nickname
    }
}

#[derive(Debug)]
pub(crate) struct Video {
    pub(crate) id: String,
    pub(crate) unique_user_id: String,
    pub(crate) nickname: String,
    pub(crate) download_address: String,
    pub(crate) description: String,
}

impl ReturnUsername for Video {
    fn username(&self) -> &str {
        &self.unique_user_id
    }
}

impl GetId for Video {
    fn id(&self) -> &str {
        &self.id
    }
}

impl ReturnTextInfo for Video {
    fn text_info(&self) -> &str {
        &self.description
    }
}

impl ReturnDataForDownload for Video {
    fn is_data_for_download(&self) -> bool {
        true
    }
    fn data(&self) -> Vec<DataForDownload> {
        vec![DataForDownload {
            url: self.download_address.clone(),
            name: self.id.clone(),
            data_type: DataType::Video,
        }]
    }
}

pub(crate) struct TiktokApi {
    secret: String,
    tiktok_domain: String,
}

impl GenerateMessage<UserInfo, Video> for TiktokApi {
    fn message(user_info: &UserInfo, video: &Video, output: &OutputType) -> String {
        let video_link = format!(
            "https://tiktok.com/@{}/video/{}",
            video.unique_user_id, video.id
        );
        match output {
            OutputType::BySubscription(stype) => {
                match stype {
                    SubscriptionType::Likes => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> liked <a href=\"{}\">video</a> from <a href=\"https://tiktok.com/@{}\">{}</a>:</i>\n\n{}",
                        user_info.unique_user_id,
                        user_info.nickname,
                        video_link,
                        video.unique_user_id,
                        video.nickname,
                        video.description
                    ),
                    SubscriptionType::Content => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> posted <a href=\"{}\">video</a></i>:\n\n{}",
                        video.unique_user_id, video.nickname, video_link, video.description
                    ),
                }
            }
        OutputType::ByLink(tguser) => {
            format!("<i>User <a href=\"tg://user?id={}\">{}</a> shared <a href=\"{}\">video</a></i>:\n\n{}", tguser.id, tguser.name, video_link, video.description)
        }
        }
    }
    fn message_format() -> Option<super::ParseMode> {
        Some(Html)
    }
}

impl TiktokApi {
    fn create_query(&self, api_name: &str, id: &str, count: u8) -> String {
        format!(
            "{}/api/{}/?username={}&count={}&key={}",
            self.tiktok_domain, api_name, id, count, self.secret
        )
    }

    async fn load_data(query: &str) -> Result<Vec<Video>, anyhow::Error> {
        let response = reqwest::get(query).await?;
        let text = response.text().await.unwrap_or("".to_string());
        let likes = serde_json::from_str::<Vec<TiktokItem>>(&text)?;
        Ok(likes
            .into_iter()
            .map(|item| Video {
                id: item.video.id,
                unique_user_id: item.author.unique_user_id,
                nickname: item.author.nickname,
                description: html_escape::encode_text(&item.description).to_string(),
                download_address: item.video.download_address,
            })
            .collect())
    }

    pub(crate) async fn send_api_new_cookie(&self, cookie: String) -> Result<(), anyhow::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!(
                "{}/api/new_cookie?key={}",
                self.tiktok_domain, self.secret
            ))
            .form(&[("cookie", &cookie)])
            .send()
            .await?;
        if self.is_alive().await {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to set a cookie or cookie is invalid"
            ))
        }
    }
}

#[async_trait]
impl ApiAlive for TiktokApi {
    async fn is_alive(&self) -> bool {
        reqwest::get(format!(
            "{}/api/status?key={}",
            self.tiktok_domain, self.secret
        ))
        .await
        .and_then(|response| Ok(response.status() == 200))
        .unwrap_or(false)
    }

    async fn try_make_alive(&self) -> Result<(), anyhow::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!(
                "{}/api/change_proxy?key={}",
                self.tiktok_domain, self.secret
            ))
            .send()
            .await?;
        Ok(())
    }
}

impl ApiName for TiktokApi {
    fn name() -> &'static str {
        "Tiktok"
    }
    fn api_type() -> Api {
        Api::Tiktok
    }
}

impl DatabaseInfoProvider for TiktokApi {
    fn user_collection_name() -> &'static str {
        "tiktokUsers"
    }

    fn chat_collection_name() -> &'static str {
        "tiktokChats"
    }

    fn content_collection_name() -> &'static str {
        "tiktokData"
    }
}

impl super::FromEnv<TiktokApi> for TiktokApi {
    fn from_env() -> TiktokApi {
        TiktokApi {
            secret: env::var("TIKTOK_API_SECRET").unwrap_or("blahblah".to_string()),
            tiktok_domain: env::var("TIKTOK_URL").unwrap_or("localhost:3000".to_string()),
        }
    }
}

#[async_trait]
impl ApiContentReceiver for TiktokApi {
    type Out = Video;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> Result<Vec<Video>, anyhow::Error> {
        let query_param = match etype {
            SubscriptionType::Content => "user_videos",
            SubscriptionType::Likes => "user_likes",
        };
        TiktokApi::load_data(&self.create_query(query_param, id, count)).await
    }

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Video> {
        let link = if regexp::TIKTOK_SHORT_LINK.is_match(link) {
            // First of all, we have to convert shortened link to full-one.
            let full_link = get_full_link(link).await?;
            log::info!("Original link({}) converted to {}", &link, &full_link);
            full_link
        } else {
            link.to_string()
        };

        for cap in regexp::TIKTOK_FULL_LINK.captures(&link) {
            let video_id = &cap[3];
            let mut data = TiktokApi::load_data(&format!(
                "{}/api/video_by_id/?video_id={}&key={}",
                &self.tiktok_domain, video_id, self.secret
            ))
            .await?;
            if let Some(video) = data.pop() {
                return Ok(video);
            }
        }
        Err(anyhow::anyhow!("Failed to fetch video by link"))
    }
}

async fn get_full_link(short_url: &str) -> anyhow::Result<String> {
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

#[async_trait]
impl ApiUserInfoReceiver for TiktokApi {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<Option<UserInfo>, anyhow::Error> {
        // Count parameter would be ignored by server
        let response = reqwest::get(self.create_query("user_info", id, 0)).await?;
        if response.status() == 404 {
            Ok(None)
        } else {
            let text = response.text().await?;
            let data = serde_json::from_str::<UserInfo>(&text)?;
            Ok(Some(data))
        }
    }
}

/*
   Structures for JSON deserialization
*/

#[derive(Deserialize, Debug)]
struct TiktokVideo {
    id: String,
    #[serde(rename = "downloadAddr")]
    download_address: String,
}

#[derive(Deserialize, Debug)]
struct TiktokAuthor {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
}

#[derive(Deserialize, Debug)]
struct TiktokItem {
    video: TiktokVideo,
    author: TiktokAuthor,
    #[serde(rename = "desc")]
    description: String,
}
