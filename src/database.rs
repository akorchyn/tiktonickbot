use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

use crate::SubscriptionType::{TiktokContent, TiktokLikes};
use anyhow;
use async_trait::async_trait;
use mongodb::{
    bson::{doc, DateTime, Document},
    options::{ClientOptions, UpdateOptions},
    Client, Database,
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) tiktok_username: String,
    pub(crate) subscribed_chats: Option<Vec<String>>,
    pub(crate) subscribed_chats_to_content: Option<Vec<String>>,
}

impl User {
    pub fn get_chats_by_subscription_type(&self, stype: SubscriptionType) -> &Option<Vec<String>> {
        match stype {
            SubscriptionType::TiktokLikes => &self.subscribed_chats,
            SubscriptionType::TiktokContent => &self.subscribed_chats_to_content,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VideoRecord {
    pub(crate) id: String,
    pub(crate) datetime: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UserStorage {
    tiktok_username: String,
    liked_videos: Option<Vec<VideoRecord>>,
    added_videos: Option<Vec<VideoRecord>>,
}

impl UserStorage {
    fn get_videos_by_subscription_type(
        &self,
        stype: SubscriptionType,
    ) -> &Option<Vec<VideoRecord>> {
        match stype {
            SubscriptionType::TiktokLikes => &self.liked_videos,
            SubscriptionType::TiktokContent => &self.added_videos,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum SubscriptionType {
    TiktokLikes,
    TiktokContent,
}

impl SubscriptionType {
    pub fn iterator() -> impl Iterator<Item = SubscriptionType> {
        [TiktokContent, TiktokLikes].iter().copied()
    }
}

impl SubscriptionType {
    fn as_subscription_string(&self) -> &'static str {
        match *self {
            SubscriptionType::TiktokLikes => "subscribed_chats",
            SubscriptionType::TiktokContent => "subscribed_chats_to_content",
        }
    }

    fn as_storage_string(&self) -> &'static str {
        match *self {
            SubscriptionType::TiktokLikes => "liked_videos",
            SubscriptionType::TiktokContent => "added_videos",
        }
    }
}

#[async_trait]
pub(crate) trait TiktokDatabaseApi {
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
    async fn get_users(&self) -> Result<Vec<User>, anyhow::Error>;
    async fn is_video_showed(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error>;
}

pub(crate) struct MongoDatabase {
    db: Database,
}

impl MongoDatabase {
    pub(crate) async fn from_connection_string(
        con_str: &str,
        database: &str,
    ) -> Result<MongoDatabase, anyhow::Error> {
        let client_options = ClientOptions::parse(con_str).await?;
        let database_client = Client::with_options(client_options)?;
        let database = database_client.database(&database);

        Ok(MongoDatabase { db: database })
    }
}

#[async_trait]
impl TiktokDatabaseApi for MongoDatabase {
    async fn subscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error> {
        let collection = self.db.collection::<Document>("users");
        let query = doc! {
            "tiktok_username": &tiktok_username,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(
                query,
                doc! {
                    "$addToSet": {stype.as_subscription_string(): &chat_id}
                },
                options,
            )
            .await?;
        Ok(())
    }

    async fn unsubscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error> {
        let collection = self.db.collection::<Document>("users");
        let query = doc! {
            "tiktok_username": &tiktok_username,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(
                query,
                doc! {
                    "$pull": {stype.as_subscription_string(): &chat_id}
                },
                options,
            )
            .await?;
        Ok(())
    }

    async fn add_video(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error> {
        let collection = self.db.collection::<Document>("userData");
        let query = doc! {
            "tiktok_username": &tiktok_username,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        let datetime: DateTime = DateTime::now();
        collection
            .update_one(
                query,
                doc! {
                    "$addToSet": {
                        stype.as_storage_string(): {
                            "id": &video_id,
                            "datetime": &datetime
                        }
                    }
                },
                options,
            )
            .await?;
        Ok(())
    }

    async fn get_users(&self) -> Result<Vec<User>, anyhow::Error> {
        let collection = self.db.collection::<User>("users");
        let cursor = collection.find(None, None).await?;
        let vector_of_results: Vec<Result<_, _>> = cursor.collect().await;
        let result: Result<Vec<_>, _> = vector_of_results.into_iter().collect();
        if let Ok(vec) = result {
            Ok(vec)
        } else {
            Err(anyhow::anyhow!("Failed to receive users"))
        }
    }

    async fn is_video_showed(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<UserStorage>("userData");
        let query = doc! {
            "tiktok_username": &tiktok_username
        };
        let option: Option<UserStorage> = collection.find_one(query, None).await?;
        if let Some(videos) = option {
            if let Some(videos) = videos.get_videos_by_subscription_type(stype) {
                return Ok(videos.into_iter().any(|video| video.id == video_id));
            }
        }
        Ok(false)
    }
}
