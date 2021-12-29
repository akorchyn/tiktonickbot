use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

use anyhow;
use mongodb::{
    bson::{doc, DateTime, Document},
    options::{ClientOptions, UpdateOptions},
    Client, Database,
};

use crate::api::{DatabaseInfoProvider, SubscriptionType};

pub(crate) trait DbCollectionForTypeRetrieval {
    fn collection<Api: DatabaseInfoProvider>() -> &'static str;
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) id: String,
    pub(crate) subscribed_chats_to_likes: Option<Vec<String>>,
    pub(crate) subscribed_chats_to_content: Option<Vec<String>>,
}

impl User {
    pub fn get_chats_by_subscription_type(&self, stype: SubscriptionType) -> &Option<Vec<String>> {
        match stype {
            SubscriptionType::Likes => &self.subscribed_chats_to_likes,
            SubscriptionType::Content => &self.subscribed_chats_to_content,
        }
    }
}

impl DbCollectionForTypeRetrieval for User {
    fn collection<Api: DatabaseInfoProvider>() -> &'static str {
        Api::user_collection_name()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct ChatInfo {
    pub(crate) chat_id: String,
    pub(crate) subscribed_for_likes_to: Option<Vec<String>>,
    pub(crate) subscribed_for_content_to: Option<Vec<String>>,
}

impl DbCollectionForTypeRetrieval for ChatInfo {
    fn collection<Api: DatabaseInfoProvider>() -> &'static str {
        Api::chat_collection_name()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ContentRecord {
    pub(crate) id: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(super) struct DataStorage {
    id: String,
    liked_content: Option<Vec<ContentRecord>>,
    processed_content: Option<Vec<ContentRecord>>,
}

impl DbCollectionForTypeRetrieval for DataStorage {
    fn collection<Api: DatabaseInfoProvider>() -> &'static str {
        Api::content_collection_name()
    }
}

impl DataStorage {
    pub(super) fn get_videos_by_subscription_type(
        &self,
        stype: SubscriptionType,
    ) -> &Option<Vec<ContentRecord>> {
        match stype {
            SubscriptionType::Likes => &self.liked_content,
            SubscriptionType::Content => &self.processed_content,
        }
    }
}

impl SubscriptionType {
    fn as_subscription_string(&self) -> &'static str {
        match *self {
            SubscriptionType::Likes => "subscribed_chats_to_likes",
            SubscriptionType::Content => "subscribed_chats_to_content",
        }
    }

    fn as_chat_string(&self) -> &'static str {
        match *self {
            SubscriptionType::Likes => "subscribed_for_likes_to",
            SubscriptionType::Content => "subscribed_for_content_to",
        }
    }

    fn as_storage_string(&self) -> &'static str {
        match *self {
            SubscriptionType::Likes => "liked_content",
            SubscriptionType::Content => "processed_content",
        }
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

    pub(crate) async fn get_collection<Api, T>(&self) -> Result<Vec<T>, anyhow::Error>
    where
        Api: DatabaseInfoProvider,
        for<'a> T:
            Deserialize<'a> + DbCollectionForTypeRetrieval + Unpin + std::marker::Send + Sync,
    {
        let collection = self.db.collection::<T>(T::collection::<Api>());
        let cursor = collection.find(None, None).await?;
        let vector_of_results: Vec<Result<_, _>> = cursor.collect().await;
        let result: Result<Vec<_>, _> = vector_of_results.into_iter().collect();
        if let Ok(vec) = result {
            Ok(vec)
        } else {
            Err(anyhow::anyhow!("Failed to receive users"))
        }
    }

    pub(crate) async fn get_chat_info<Api>(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatInfo>, anyhow::Error>
    where
        Api: DatabaseInfoProvider,
    {
        let collection = self.db.collection::<ChatInfo>(Api::chat_collection_name());
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

    pub(crate) async fn subscribe_user<Api>(
        &self,
        id: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>
    where
        Api: DatabaseInfoProvider,
    {
        self.push_to_collection(
            Api::user_collection_name(),
            doc! {
                "id": id,
            },
            doc! {
                stype.as_subscription_string(): chat_id
            },
        )
        .await?;
        self.push_to_collection(
            Api::chat_collection_name(),
            doc! {
                "chat_id": chat_id,
            },
            doc! {
                stype.as_chat_string(): id
            },
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn unsubscribe_user<Api>(
        &self,
        id: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>
    where
        Api: DatabaseInfoProvider,
    {
        self.pull_from_collection(
            Api::user_collection_name(),
            doc! {
                "id": id,
            },
            doc! {
                stype.as_subscription_string(): chat_id
            },
        )
        .await?;
        self.pull_from_collection(
            Api::chat_collection_name(),
            doc! {
                "chat_id": chat_id,
            },
            doc! {
                stype.as_chat_string(): id
            },
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn add_content<Api>(
        &self,
        content_id: &str,
        user_id: &str,
        stype: SubscriptionType,
    ) -> Result<(), anyhow::Error>
    where
        Api: DatabaseInfoProvider,
    {
        let datetime: DateTime = DateTime::now();
        self.push_to_collection(
            Api::content_collection_name(),
            doc! {
                "id": &user_id,
            },
            doc! {stype.as_storage_string(): {
                "id": &content_id,
                "datetime": &datetime
            }},
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn is_content_showed<Api>(
        &self,
        video_id: &str,
        user_id: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error>
    where
        Api: DatabaseInfoProvider,
    {
        let collection = self
            .db
            .collection::<DataStorage>(Api::content_collection_name());
        let query = doc! {
            "id": &user_id
        };
        let option: Option<DataStorage> = collection.find_one(query, None).await?;
        if let Some(videos) = option {
            if let Some(videos) = videos.get_videos_by_subscription_type(stype) {
                return Ok(videos.into_iter().any(|video| video.id == video_id));
            }
        }
        Ok(false)
    }

    pub(crate) async fn is_user_subscribed<Api: DatabaseInfoProvider>(
        &self,
        user_id: &str,
        chat_id: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<User>(Api::user_collection_name());
        let query = doc! {
            "id": &user_id
        };
        let option: Option<User> = collection.find_one(query, None).await?;
        if let Some(user) = option {
            if let Some(chats) = user.get_chats_by_subscription_type(stype) {
                return Ok(chats.into_iter().any(|id| chat_id == id));
            }
        }
        Ok(false)
    }

    pub(crate) async fn is_user_exist<Api: DatabaseInfoProvider>(
        &self,
        user_id: &str,
        stype: SubscriptionType,
    ) -> Result<bool, anyhow::Error> {
        let collection = self.db.collection::<User>(Api::user_collection_name());
        let query = doc! {
            "id": &user_id
        };
        let option: Option<User> = collection.find_one(query, None).await?;
        if let Some(user) = option {
            if let Some(chats) = user.get_chats_by_subscription_type(stype) {
                return Ok(!chats.is_empty());
            }
        }
        Ok(false)
    }
}
