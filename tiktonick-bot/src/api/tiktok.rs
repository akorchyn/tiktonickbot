use crate::api::{
    api_requests, Api, ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload, DataType,
    DatabaseInfoProvider, GenerateMessage, GetId, OutputType, ReturnDataForDownload,
    ReturnTextInfo, ReturnUserInfo, ReturnUsername, SubscriptionType,
};
use crate::regexp;

use async_trait::async_trait;
use serde::{self, Deserialize};
use teloxide::types::ParseMode::Html;

#[derive(Deserialize, Debug)]
pub(crate) struct UserInfo {
    #[serde(rename = "uniqueId")]
    pub(crate) unique_user_id: String,
    #[serde(rename = "nickname")]
    pub(crate) nickname: String,
}

impl ReturnUserInfo for UserInfo {
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

#[derive(Debug)]
pub(crate) struct Video {
    pub(crate) id: String,
    pub(crate) unique_user_id: String,
    pub(crate) nickname: String,
    pub(crate) download_address: String,
    pub(crate) description: String,
}

impl ReturnUsername for Video {
    fn username(&self) -> &str {
        &self.unique_user_id
    }
}

impl GetId for Video {
    fn id(&self) -> &str {
        &self.id
    }
}

impl ReturnTextInfo for Video {
    fn text_info(&self) -> &str {
        &self.description
    }
}

impl ReturnDataForDownload for Video {
    fn is_data_for_download(&self) -> bool {
        true
    }
    fn data(&self) -> Vec<DataForDownload> {
        vec![DataForDownload {
            url: self.download_address.clone(),
            name: self.id.clone(),
            data_type: DataType::Video,
        }]
    }
}

pub(crate) struct TiktokAPI {
    api_url_generator: api_requests::ApiUrlGenerator,
}

impl GenerateMessage<UserInfo, Video> for TiktokAPI {
    fn message(user_info: &UserInfo, video: &Video, output: &OutputType) -> String {
        let video_link = format!(
            "https://tiktok.com/@{}/video/{}",
            video.unique_user_id, video.id
        );
        match output {
            OutputType::BySubscription(stype) => {
                match stype {
                    SubscriptionType::Subscription1 => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> liked <a href=\"{}\">video</a> from <a href=\"https://tiktok.com/@{}\">{}</a>:</i>\n\n{}",
                        user_info.unique_user_id,
                        user_info.nickname,
                        video_link,
                        video.unique_user_id,
                        video.nickname,
                        video.description
                    ),
                    SubscriptionType::Subscription2 => format!(
                        "<i>User <a href=\"https://tiktok.com/@{}\">{}</a> posted <a href=\"{}\">video</a></i>:\n\n{}",
                        video.unique_user_id, video.nickname, video_link, video.description
                    ),
                }
            }
        OutputType::ByLink(tguser) => {
            format!("<i>User <a href=\"tg://user?id={}\">{}</a> shared <a href=\"{}\">video</a></i>:\n\n{}", tguser.id, tguser.name, video_link, video.description)
        }
        }
    }
    fn message_format() -> Option<super::ParseMode> {
        Some(Html)
    }
}

impl TiktokAPI {
    async fn load_data(&self, url: &str) -> Result<Vec<Video>, anyhow::Error> {
        let likes = self
            .api_url_generator
            .get_data::<Vec<TiktokItem>>(url)
            .await?;
        log::info!("Received item. Item status is {}", likes.is_some());
        let likes = likes.unwrap_or_default();
        log::info!(
            "Received {} videos from request for {} url",
            likes.len(),
            url
        );
        Ok(likes
            .into_iter()
            .map(|item| Video {
                id: item.video.id,
                unique_user_id: item.author.unique_user_id,
                nickname: item.author.nickname,
                description: html_escape::encode_text(&item.description).to_string(),
                download_address: item.video.download_address,
            })
            .collect())
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
impl ApiContentReceiver for TiktokAPI {
    type Out = Video;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> Result<Vec<Video>, anyhow::Error> {
        let api = match etype {
            SubscriptionType::Subscription2 => "videos",
            SubscriptionType::Subscription1 => "likes",
        };
        self.load_data(
            &self
                .api_url_generator
                .get_user_content_by_type(id, api, count),
        )
        .await
    }

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Video> {
        let link = if regexp::TIKTOK_SHORT_LINK.is_match(link) {
            // First of all, we have to convert shortened link to full-one.
            let full_link = api_requests::get_full_link(link).await?;
            log::info!("Original link({}) converted to {}", &link, &full_link);
            full_link
        } else {
            link.to_string()
        };

        if let Some(cap) = regexp::TIKTOK_FULL_LINK.captures(&link) {
            let video_id = &cap[3];
            let mut data = self
                .load_data(&self.api_url_generator.get_content_by_id(video_id))
                .await?;
            if let Some(video) = data.pop() {
                return Ok(video);
            }
        }
        Err(anyhow::anyhow!("Failed to fetch video by link"))
    }
}

#[async_trait]
impl ApiUserInfoReceiver for TiktokAPI {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<Option<UserInfo>, anyhow::Error> {
        Ok(self
            .api_url_generator
            .get_data::<UserInfo>(&self.api_url_generator.get_user_info(&id))
            .await?)
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
struct TiktokAuthor {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
}

#[derive(Deserialize, Debug)]
struct TiktokItem {
    video: TiktokVideo,
    author: TiktokAuthor,
    #[serde(rename = "desc")]
    description: String,
}
