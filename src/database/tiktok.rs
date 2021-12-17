use super::CollectionReturn;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub(crate) use crate::api::tiktok::SubscriptionType;

#[async_trait]
pub(crate) trait DatabaseApi {
    async fn subscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>;

    async fn unsubscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>;
    async fn add_video(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>;
    async fn is_video_showed(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error>;
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) tiktok_username: String,
    pub(crate) subscribed_chats: Option<Vec<String>>,
    pub(crate) subscribed_chats_to_content: Option<Vec<String>>,
}

impl User {
    pub fn get_chats_by_subscription_type(&self, stype: SubscriptionType) -> &Option<Vec<String>> {
        match stype {
            SubscriptionType::Likes => &self.subscribed_chats,
            SubscriptionType::CreatedVideos => &self.subscribed_chats_to_content,
        }
    }
}

impl CollectionReturn for User {
    fn collection_name() -> &'static str {
        "users"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VideoRecord {
    pub(crate) id: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(super) struct UserStorage {
    tiktok_username: String,
    liked_videos: Option<Vec<VideoRecord>>,
    added_videos: Option<Vec<VideoRecord>>,
}

impl UserStorage {
    pub(super) fn get_videos_by_subscription_type(
        &self,
        stype: SubscriptionType,
    ) -> &Option<Vec<VideoRecord>> {
        match stype {
            SubscriptionType::Likes => &self.liked_videos,
            SubscriptionType::CreatedVideos => &self.added_videos,
        }
    }
}

impl SubscriptionType {
    pub(super) fn as_subscription_string(&self) -> &'static str {
        match *self {
            SubscriptionType::Likes => "subscribed_chats",
            SubscriptionType::CreatedVideos => "subscribed_chats_to_content",
        }
    }

    pub(super) fn as_storage_string(&self) -> &'static str {
        match *self {
            SubscriptionType::Likes => "liked_videos",
            SubscriptionType::CreatedVideos => "added_videos",
        }
    }
}
