pub(crate) mod tiktok;
pub(crate) mod twitter;

use anyhow;
use async_trait::async_trait;
use serde::Deserialize;

use teloxide::types::ParseMode;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum Api {
    Tiktok,
    Twitter,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum SubscriptionType {
    Likes,
    Content,
}

impl SubscriptionType {
    pub(crate) fn iterator() -> impl Iterator<Item = SubscriptionType> {
        [SubscriptionType::Content, SubscriptionType::Likes]
            .iter()
            .copied()
    }
}

pub(crate) trait ApiName {
    fn name() -> &'static str;
    fn api_type() -> Api;
}

pub(crate) trait GetId {
    fn id(&self) -> &str;
}

#[async_trait]
pub(crate) trait ApiAlive {
    async fn is_alive(&self) -> bool;
    async fn try_make_alive(&self) -> Result<(), anyhow::Error>;
}

#[async_trait]
pub(crate) trait ApiUserInfoReceiver {
    type Out;
    async fn get_user_info(&self, id: &str) -> Result<Option<Self::Out>, anyhow::Error>;
}

#[async_trait]
pub(crate) trait ApiContentReceiver {
    type Out;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> Result<Vec<Self::Out>, anyhow::Error>;
}

pub(crate) trait FromEnv<Api> {
    fn from_env() -> Api;
}

pub(crate) trait ReturnUserInfo {
    fn id(&self) -> &str;
    fn username(&self) -> &str;
    fn nickname(&self) -> &str;
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) enum DataType {
    Image,
    Video,
}

impl DataType {
    pub(crate) fn to_extension(&self) -> &'static str {
        match *self {
            DataType::Video => "mp4",
            DataType::Image => "jpg",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DataForDownload {
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) data_type: DataType,
}

pub(crate) trait ReturnDataForDownload {
    fn is_data_for_download(&self) -> bool;
    fn data(&self) -> Vec<DataForDownload>;
}

pub(crate) trait ReturnTextInfo {
    fn text_info(&self) -> &str;
}

pub(crate) trait GenerateSubscriptionMessage<UserInfo, Content> {
    fn subscription_message(
        user_info: &UserInfo,
        content: &Content,
        stype: SubscriptionType,
    ) -> String;
    fn subscription_format() -> Option<ParseMode>;
}

pub(crate) trait DatabaseInfoProvider {
    fn user_collection_name() -> &'static str;
    fn chat_collection_name() -> &'static str;
    fn content_collection_name() -> &'static str;
}
