use reqwest;

use std::env;

use crate::api::{ApiContentReceiver, ApiUserInfoReceiver};
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

#[derive(Debug)]
pub(crate) struct Video {
    pub(crate) id: String,
    pub(crate) unique_user_id: String,
    pub(crate) nickname: String,
    pub(crate) download_address: String,
    pub(crate) description: String,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum SubscriptionType {
    Likes,
    CreatedVideos,
}

impl SubscriptionType {
    pub(crate) fn iterator() -> impl Iterator<Item = SubscriptionType> {
        [SubscriptionType::CreatedVideos, SubscriptionType::Likes]
            .iter()
            .copied()
    }

    fn to_query_param(&self) -> &'static str {
        match *self {
            SubscriptionType::CreatedVideos => "user_videos",
            SubscriptionType::Likes => "user_likes",
        }
    }
}

pub(crate) struct TiktokApi {
    secret: String,
    tiktok_domain: String,
}

impl TiktokApi {
    pub(crate) fn from_env() -> TiktokApi {
        TiktokApi {
            secret: env::var("TIKTOK_API_SECRET").unwrap_or("blahblah".to_string()),
            tiktok_domain: env::var("TIKTOK_URL").unwrap_or("localhost:3000".to_string()),
        }
    }

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
}

#[async_trait]
impl ApiContentReceiver<SubscriptionType, Video> for TiktokApi {
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> Result<Vec<Video>, anyhow::Error> {
        TiktokApi::load_data(&self.create_query(etype.to_query_param(), id, count)).await
    }
}

#[async_trait]
impl ApiUserInfoReceiver<UserInfo> for TiktokApi {
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
