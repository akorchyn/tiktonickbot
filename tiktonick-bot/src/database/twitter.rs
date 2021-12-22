// use super::CollectionReturn;

// use async_trait::async_trait;
// use serde::{Deserialize, Serialize};

// #[async_trait]
// pub(crate) trait DatabaseApi {
//     async fn subscribe_user(
//         &self,
//         tiktok_username: &str,
//         chat_id: &str,
//         stype: SubscriptionType,
//     ) -> Result<(), anyhow::Error>;

//     async fn unsubscribe_user(
//         &self,
//         tiktok_username: &str,
//         chat_id: &str,
//         stype: SubscriptionType,
//     ) -> Result<(), anyhow::Error>;
//     async fn add_video(
//         &self,
//         video_id: &str,
//         tiktok_username: &str,
//         stype: SubscriptionType,
//     ) -> Result<(), anyhow::Error>;
//     async fn is_video_showed(
//         &self,
//         video_id: &str,
//         tiktok_username: &str,
//         stype: SubscriptionType,
//     ) -> Result<bool, anyhow::Error>;
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub(crate) struct User {
//     pub(crate) id: String,
//     pub(crate) name: String,
//     pub(crate) username: String,
//     pub(crate) subscribed_chats_to_likes: Option<Vec<String>>,
// }

// impl CollectionReturn for User {
//     fn collection_name() -> &'static str {
//         "twitter_users"
//     }
// }

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub(crate) struct TweetAttachments {
//     media_keys: Vec<String>
// }

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub(crate) struct Tweet {
//     id: String,
//     text: String,
//     attachments: Option<TweetAttachments>
// }

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub(super) struct Tweets {
//     user_id: Option<String>,
//     data: Vec<Tweet>
// }
