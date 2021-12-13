use anyhow;
use async_trait::async_trait;
use futures::stream::{StreamExt};
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, UpdateOptions},
    Client, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) tiktok_username: String,
    pub(crate) subscribed_chats: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UserVideos {
    pub(crate) tiktok_username: String,
    pub(crate) liked_videos: Vec<String>,
}

#[async_trait]
pub(crate) trait TiktokDatabaseApi {
    async fn subscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
    ) -> Result<(), anyhow::Error>;
    async fn unsubscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
    ) -> Result<(), anyhow::Error>;
    async fn add_video(
        &self,
        video_id: &str,
        tiktok_username: &str,
    ) -> Result<(), anyhow::Error>;
    async fn get_users(&self) -> Result<Vec<User>, anyhow::Error>;
    async fn is_video_showed(
        &self,
        video_id: &str,
        tiktok_username: &str,
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

        Ok(MongoDatabase {
            db: database,
        })
    }
}

#[async_trait]
impl TiktokDatabaseApi for MongoDatabase {
    async fn subscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
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
                    "$addToSet": {"subscribed_chats": &chat_id}
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
                    "$pull": {"subscribed_chats": &chat_id}
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
    ) -> Result<(), anyhow::Error> {
        let collection = self.db.collection::<Document>("userData");
        let query = doc! {
            "tiktok_username": &tiktok_username,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(
                query,
                doc! {
                    "$addToSet": {"liked_videos": &video_id}
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
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<UserVideos>("userData");
        let query = doc! {
            "tiktok_username": &tiktok_username
        };
        let option:Option<UserVideos> = collection.find_one(query, None).await?;
        if let Some(videos) = option {
            Ok(videos.liked_videos.contains(&video_id.to_string()))
        } else {
            Ok(false)
        }
    }
}
