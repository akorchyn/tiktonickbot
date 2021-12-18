use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct ChatInfo {
    pub(crate) chat_id: String,
    pub(crate) subscribed_for_likes_to: Vec<String>,
    pub(crate) subscribed_for_content_to: Vec<String>,
}

impl CollectionReturn for ChatInfo {
    fn collection_name() -> &'static str {
        "chats"
    }
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

    pub(crate) async fn get_collection<T>(&self) -> Result<Vec<T>, anyhow::Error>
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

    pub(crate) async fn get_chat_info(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatInfo>, anyhow::Error> {
        let collection = self.db.collection::<ChatInfo>(ChatInfo::collection_name());
        let query = doc! {
            "chat_id": chat_id
        };
        Ok(collection.find_one(query, None).await?)
    }

    async fn op_on_collection(
        &self,
        op: &str,
        collection_name: &str,
        query: Document,
        data: Document,
    ) -> Result<(), anyhow::Error> {
        let collection = self.db.collection::<Document>(&collection_name);
        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(
                query,
                doc! {
                    op: data
                },
                options,
            )
            .await?;
        Ok(())
    }

    async fn push_to_collection(
        &self,
        collection_name: &str,
        query: Document,
        data: Document,
    ) -> Result<(), anyhow::Error> {
        self.op_on_collection("$addToSet", collection_name, query, data)
            .await
    }

    async fn pull_from_collection(
        &self,
        collection_name: &str,
        query: Document,
        data: Document,
    ) -> Result<(), anyhow::Error> {
        self.op_on_collection("$pull", collection_name, query, data)
            .await
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
        self.push_to_collection(
            "users",
            doc! {
                "tiktok_username": tiktok_username,
            },
            doc! {
                stype.as_subscription_string(): chat_id
            },
        )
        .await?;
        self.push_to_collection(
            "chats",
            doc! {
                "chat_id": chat_id,
            },
            doc! {
                stype.as_chat_string(): tiktok_username
            },
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
        self.pull_from_collection(
            "users",
            doc! {
                "tiktok_username": tiktok_username,
            },
            doc! {
                stype.as_subscription_string(): chat_id
            },
        )
        .await?;
        self.pull_from_collection(
            "chats",
            doc! {
                "chat_id": chat_id,
            },
            doc! {
                stype.as_chat_string(): tiktok_username
            },
        )
        .await?;
        Ok(())
    }

    async fn add_content(
        &self,
        content_id: &str,
        user_id: &str,
        stype: tiktok::SubscriptionType,
    ) -> Result<(), anyhow::Error> {
        let datetime: DateTime = DateTime::now();
        self.push_to_collection(
            "userData",
            doc! {
                "tiktok_username": &user_id,
            },
            doc! {stype.as_storage_string(): {
                "id": &content_id,
                "datetime": &datetime
            }},
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

    async fn is_user_subscribed(
        &self,
        user_id: &str,
        chat_id: &str,
        stype: tiktok::SubscriptionType,
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<tiktok::User>("users");
        let query = doc! {
            "tiktok_username": &user_id
        };
        let option: Option<tiktok::User> = collection.find_one(query, None).await?;
        if let Some(user) = option {
            if let Some(chats) = user.get_chats_by_subscription_type(stype) {
                return Ok(chats.into_iter().any(|id| chat_id == id));
            }
        }
        Ok(false)
    }
}
