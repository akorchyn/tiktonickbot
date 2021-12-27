use reqwest;

use std::env;

use crate::api::{
    ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload, DataType,
    DatabaseInfoProvider, GenerateSubscriptionMessage, GetId, ReturnDataForDownload,
    ReturnTextInfo, ReturnUserInfo, SubscriptionType,
};
use anyhow;
use async_trait::async_trait;
use serde::{self, Deserialize};
use serde_json;

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
    fn username(&self) -> &str {
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

impl GenerateSubscriptionMessage<UserInfo, Video> for TiktokApi {
    fn subscription_message(
        user_info: &UserInfo,
        video: &Video,
        stype: SubscriptionType,
    ) -> String {
        match stype {
            SubscriptionType::Likes => format!(
                "User {} aka {} liked video from {} aka {}.\n\nDescription:\n{}",
                user_info.unique_user_id,
                user_info.nickname,
                video.unique_user_id,
                video.nickname,
                video.description
            ),
            SubscriptionType::Content => format!(
                "User {} aka {} posted video.\n\nDescription:\n{}",
                video.unique_user_id, video.nickname, video.description
            ),
        }
    }
    fn subscription_format() -> Option<super::ParseMode> {
        None
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
                description: item.description,
                download_address: item.video.download_address,
            })
            .collect())
    }

    pub(crate) async fn check_alive(&self) -> bool {
        reqwest::get(format!(
            "{}/api/status?key={}",
            self.tiktok_domain, self.secret
        ))
        .await
        .and_then(|response| Ok(response.status() == 200))
        .unwrap_or(false)
    }

    pub(crate) async fn change_proxy(&self) -> String {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "{}/api/change_proxy?key={}",
                self.tiktok_domain, self.secret
            ))
            .send()
            .await;
        if let Ok(response) = response {
            response.text().await.unwrap_or("Failed".to_string())
        } else {
            "Failed".to_string()
        }
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
        if self.check_alive().await {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to set a cookie or cookie is invalid"
            ))
        }
    }
}

impl ApiName for TiktokApi {
    fn name() -> &'static str {
        "Tiktok"
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
}

#[async_trait]
impl ApiUserInfoReceiver for TiktokApi {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<UserInfo, anyhow::Error> {
        // Count parameter would be ignored by server
        let response = reqwest::get(self.create_query("user_info", id, 0)).await?;
        let text = response.text().await?;
        let data = serde_json::from_str::<UserInfo>(&text)?;
        Ok(data)
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
