use crate::api::{
    api_data_fetcher, Api, ApiName, DataForDownload, DataType, DatabaseInfoProvider, GetId,
    OutputType, PrepareDescription, ReturnDataForDownload, ReturnUserInfo, ReturnUsername,
    SubscriptionType,
};
use crate::regexp;

use crate::api::api_data_fetcher::ApiDataFetcher;
use crate::api::default_loaders::DefaultDataFetcherInfo;
use crate::common::description_builder::{ActionType, DescriptionBuilder};
use async_trait::async_trait;
use serde::{self, Deserialize};

pub(crate) struct TiktokAPI {
    data_fetcher: api_data_fetcher::ApiDataFetcher,
}

impl PrepareDescription<TiktokAuthor, TiktokItem> for TiktokAPI {
    fn prepare_description(
        user_info: &TiktokAuthor,
        item: &TiktokItem,
        output: &OutputType,
    ) -> DescriptionBuilder {
        let description = html_escape::encode_text(&item.description).to_string();
        let video_link = format!(
            "https://tiktok.com/@{}/video/{}",
            item.author.unique_user_id, item.video.id
        );
        let user_link = |user: &str| format!("https://tiktok.com/@{}", user);

        let mut builder = DescriptionBuilder::new();
        match output {
            OutputType::BySubscription(stype) => match stype {
                SubscriptionType::Subscription1 => builder.action(ActionType::Liked).from(
                    &item.author.nickname,
                    &user_link(&item.author.unique_user_id),
                ),
                SubscriptionType::Subscription2 => builder.action(ActionType::Posted),
            }
            .who(&user_info.nickname, &user_link(&user_info.unique_user_id)),
            OutputType::ByLink(tguser) => builder
                .who(&tguser.name, &tguser.user_link())
                .action(ActionType::Shared)
                .from(
                    &item.author.nickname,
                    &user_link(&item.author.unique_user_id),
                ),
        }
        .content("video", &video_link)
        .description(description);

        builder
    }
}

impl ApiName for TiktokAPI {
    fn name() -> &'static str {
        "Tiktok"
    }
    fn api_type() -> Api {
        Api::Tiktok
    }
}

impl DatabaseInfoProvider for TiktokAPI {
    fn user_collection_name() -> &'static str {
        "tiktokUsers"
    }

    fn chat_collection_name() -> &'static str {
        "tiktokChats"
    }

    fn content_collection_name() -> &'static str {
        "tiktokData"
    }
}

impl super::FromEnv<TiktokAPI> for TiktokAPI {
    fn from_env() -> TiktokAPI {
        TiktokAPI {
            data_fetcher: api_data_fetcher::ApiDataFetcher::from_env(TiktokAPI::api_type()),
        }
    }
}

#[async_trait]
impl DefaultDataFetcherInfo for TiktokAPI {
    type UserInfo = TiktokAuthor;
    type Content = TiktokItem;

    fn data_fetcher(&self) -> &ApiDataFetcher {
        &self.data_fetcher
    }

    fn subscription_type_to_api_type(s: SubscriptionType) -> &'static str {
        match s {
            SubscriptionType::Subscription1 => "likes",
            SubscriptionType::Subscription2 => "videos",
        }
    }

    async fn get_content_id_from_url(&self, url: &str) -> Option<String> {
        let link = if regexp::TIKTOK_SHORT_LINK.is_match(url) {
            // First of all, we have to convert shortened link to full-one.
            let full_link = api_data_fetcher::get_full_link(url).await.ok()?;
            log::info!("Original link({}) converted to {}", &url, &full_link);
            full_link
        } else {
            url.to_string()
        };

        regexp::TIKTOK_FULL_LINK
            .captures(&link)
            .and_then(|cap| cap.get(3))
            .map(|m| m.as_str().to_string())
    }
}

/*
   Structures for JSON deserialization
*/

#[derive(Deserialize, Debug)]
struct TiktokVideo {
    id: String,
    #[serde(rename = "downloadAddr")]
    download_address: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct TiktokAuthor {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
}

impl ReturnUserInfo for TiktokAuthor {
    fn id(&self) -> &str {
        &self.unique_user_id
    }
    fn unique_user_name(&self) -> &str {
        &self.unique_user_id
    }
    fn nickname(&self) -> &str {
        &self.nickname
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct TiktokItem {
    video: TiktokVideo,
    author: TiktokAuthor,
    #[serde(rename = "desc")]
    description: String,
}

impl ReturnUsername for TiktokItem {
    fn username(&self) -> &str {
        self.author.unique_user_name()
    }
}

impl GetId for TiktokItem {
    fn id(&self) -> &str {
        &self.video.id
    }
}

impl ReturnDataForDownload for TiktokItem {
    fn is_data_for_download(&self) -> bool {
        true
    }
    fn data(&self) -> Vec<DataForDownload> {
        vec![DataForDownload {
            url: self.video.download_address.clone(),
            name: self.video.id.clone(),
            data_type: DataType::Video,
        }]
    }
}
