use crate::api::api_data_fetcher::ApiDataFetcher;
use crate::api::{ApiContentReceiver, ApiUserInfoReceiver, SubscriptionType};

use crate::api::api_data_fetcher::Request;
use async_trait::async_trait;
use serde::Deserialize;

#[async_trait]
pub(crate) trait DefaultDataFetcherInfo {
    type UserInfo;
    type Content;
    fn data_fetcher(&self) -> &ApiDataFetcher;
    fn subscription_type_to_api_type(_: SubscriptionType) -> &'static str;
    async fn get_content_id_from_url(&self, url: &str) -> Option<String>;
}

#[async_trait]
impl<T> ApiUserInfoReceiver for T
where
    T: DefaultDataFetcherInfo + Send + Sync,
    for<'a> T::UserInfo: Deserialize<'a>,
{
    type Out = T::UserInfo;
    async fn get_user_info(&self, id: &str) -> anyhow::Result<Option<Self::Out>> {
        self.data_fetcher()
            .get_data::<T::UserInfo>(Request::UserData(&id))
            .await
    }
}

#[async_trait]
impl<T> ApiContentReceiver for T
where
    T: DefaultDataFetcherInfo + Send + Sync,
    for<'a> T::Content: Deserialize<'a>,
{
    type Out = T::Content;

    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> anyhow::Result<Vec<Self::Out>> {
        Ok(self
            .data_fetcher()
            .get_data::<Vec<T::Content>>(Request::Content(
                id,
                T::subscription_type_to_api_type(etype),
                count,
            ))
            .await?
            .unwrap_or_default())
    }

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Option<Self::Out>> {
        let id = self.get_content_id_from_url(link).await;
        if let Some(id) = id {
            Ok(self
                .data_fetcher()
                .get_data::<Self::Out>(Request::ContentById(&id))
                .await?)
        } else {
            Ok(None)
        }
    }
}
