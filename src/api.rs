pub(crate) mod tiktok;
pub(crate) mod twitter;

use anyhow;
use async_trait::async_trait;
use serde::Deserialize;

use teloxide::types::ParseMode;

#[async_trait]
pub(crate) trait ApiUserInfoReceiver {
    type Out;
    async fn get_user_info(&self, id: &str) -> Result<Self::Out, anyhow::Error>;
}

#[async_trait]
pub(crate) trait ApiContentReceiver {
    type Out;
    type ContentType;

    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: Self::ContentType,
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
    fn subscription_message(&self, user_info: &UserInfo, content: &Content) -> String;
    fn subscription_format(&self) -> Option<ParseMode>;
}
