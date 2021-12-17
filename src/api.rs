pub(crate) mod tiktok;

use anyhow;
use async_trait::async_trait;

#[async_trait]
pub(crate) trait ApiUserInfoReceiver<Out = Self> {
    async fn get_user_info(&self, id: &str) -> Result<Out, anyhow::Error>;
}

#[async_trait]
pub(crate) trait ApiContentReceiver<ContentType, Out = Self> {
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: ContentType,
    ) -> Result<Vec<Out>, anyhow::Error>;
}
