use futures::stream::StreamExt;
use serde::Deserialize;

use anyhow;
use async_trait::async_trait;
use mongodb::{
    bson::{doc, DateTime, Document},
    options::{ClientOptions, UpdateOptions},
    Client, Database,
};

pub(crate) mod tiktok;
pub(crate) mod twitter;

pub(crate) trait CollectionReturn {
    fn collection_name() -> &'static str;
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

    pub(crate) async fn get_users<T>(&self) -> Result<Vec<T>, anyhow::Error>
    where
        for<'a> T: Deserialize<'a> + CollectionReturn + Unpin + std::marker::Send + Sync,
    {
        let collection = self.db.collection::<T>(T::collection_name());
        let cursor = collection.find(None, None).await?;
        let vector_of_results: Vec<Result<_, _>> = cursor.collect().await;
        let result: Result<Vec<_>, _> = vector_of_results.into_iter().collect();
        if let Ok(vec) = result {
            Ok(vec)
        } else {
            Err(anyhow::anyhow!("Failed to receive users"))
        }
    }
}

#[async_trait]
impl tiktok::DatabaseApi for MongoDatabase {
    async fn subscribe_user(
        &self,
        tiktok_username: &str,
        chat_id: &str,
        stype: tiktok::SubscriptionType,
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
        stype: tiktok::SubscriptionType,
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
        stype: tiktok::SubscriptionType,
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

    async fn is_video_showed(
        &self,
        video_id: &str,
        tiktok_username: &str,
        stype: tiktok::SubscriptionType,
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<tiktok::UserStorage>("userData");
        let query = doc! {
            "tiktok_username": &tiktok_username
        };
        let option: Option<tiktok::UserStorage> = collection.find_one(query, None).await?;
        if let Some(videos) = option {
            if let Some(videos) = videos.get_videos_by_subscription_type(stype) {
                return Ok(videos.into_iter().any(|video| video.id == video_id));
            }
        }
        Ok(false)
    }
}
