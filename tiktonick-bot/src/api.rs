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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TelegramUser {
    pub(crate) name: String,
    pub(crate) id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OutputType {
    BySubscription(SubscriptionType),
    ByLink(TelegramUser),
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

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Self::Out>;
}

pub(crate) trait FromEnv<Api> {
    fn from_env() -> Api;
}

pub(crate) trait ReturnUsername {
    fn username(&self) -> &str;
}

pub(crate) trait ReturnUserInfo {
    fn id(&self) -> &str;
    fn unique_user_name(&self) -> &str;
    fn nickname(&self) -> &str;
}

impl<T> ReturnUsername for T
where
    T: ReturnUserInfo,
{
    fn username(&self) -> &str {
        self.unique_user_name()
    }
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

pub(crate) trait GenerateMessage<UserInfo, Content> {
    fn message(user_info: &UserInfo, content: &Content, stype: &OutputType) -> String;
    fn message_format() -> Option<ParseMode>;
}

pub(crate) trait DatabaseInfoProvider {
    fn user_collection_name() -> &'static str;
    fn chat_collection_name() -> &'static str;
    fn content_collection_name() -> &'static str;
}
