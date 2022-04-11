use crate::api::{
    api_requests, Api, ApiName, DataForDownload, DataType, DatabaseInfoProvider, GenerateMessage,
    GetId, OutputType, ReturnDataForDownload, ReturnUserInfo, ReturnUsername, SubscriptionType,
};
use crate::regexp;

use crate::api::api_requests::ApiUrlGenerator;
use crate::api::default_loaders::DefaultDataFetcherInfo;
use async_trait::async_trait;
use serde::{self, Deserialize};
use teloxide::types::ParseMode::Html;

pub(crate) struct TiktokAPI {
    api_url_generator: api_requests::ApiUrlGenerator,
}

impl GenerateMessage<TiktokAuthor, TiktokItem> for TiktokAPI {
    fn message(user_info: &TiktokAuthor, item: &TiktokItem, output: &OutputType) -> String {
        let description = html_escape::encode_text(&item.description);
        let video_link = format!(
            "https://tiktok.com/@{}/video/{}",
            item.author.unique_user_id, item.video.id
        );
        match output {
            OutputType::BySubscription(stype) => {
                match stype {
                    SubscriptionType::Subscription1 => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> liked <a href=\"{}\">video</a> from <a href=\"https://tiktok.com/@{}\">{}</a>:</i>\n\n{}",
                        user_info.unique_user_id,
                        user_info.nickname,
                        video_link,
                        item.author.unique_user_id,
                        item.author.nickname,
                        description
                    ),
                    SubscriptionType::Subscription2 => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> posted <a href=\"{}\">video</a></i>:\n\n{}",
                        item.author.unique_user_id, item.author.nickname, video_link, description
                    ),
                }
            }
        OutputType::ByLink(tguser) => {
            format!("<i>User <a href=\"tg://user?id={}\">{}</a> shared <a href=\"{}\">video</a></i>:\n\n{}", tguser.id, tguser.name, video_link, description)
        }
        }
    }
    fn message_format() -> super::ParseMode {
        Html
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
            api_url_generator: api_requests::ApiUrlGenerator::from_env("tiktok".to_string()),
        }
    }
}

#[async_trait]
impl DefaultDataFetcherInfo for TiktokAPI {
    type UserInfo = TiktokAuthor;
    type Content = TiktokItem;

    fn api_url_generator(&self) -> &ApiUrlGenerator {
        &self.api_url_generator
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
            let full_link = api_requests::get_full_link(url).await.ok()?;
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
        &self.author.unique_user_name()
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
