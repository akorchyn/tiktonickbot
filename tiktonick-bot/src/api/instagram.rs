use serde::{self, Deserialize};

use crate::api::api_data_fetcher::ApiDataFetcher;
use crate::api::default_loaders::DefaultDataFetcherInfo;
use crate::api::{
    api_data_fetcher, Api, ApiName, DataForDownload, DataType, DatabaseInfoProvider, FromEnv,
    GetId, OutputType, PrepareDescription, ReturnDataForDownload, ReturnUserInfo, SubscriptionType,
};
use crate::common::description_builder::{ActionType, DescriptionBuilder};
use crate::regexp;
use async_trait::async_trait;

pub(crate) struct InstagramAPI {
    data_fetcher: api_data_fetcher::ApiDataFetcher,
}

impl FromEnv<InstagramAPI> for InstagramAPI {
    fn from_env() -> Self {
        let data_fetcher = api_data_fetcher::ApiDataFetcher::from_env(InstagramAPI::api_type());
        Self { data_fetcher }
    }
}

impl ApiName for InstagramAPI {
    fn name() -> &'static str {
        "Instagram"
    }
    fn api_type() -> Api {
        Api::Instagram
    }
}

impl DatabaseInfoProvider for InstagramAPI {
    fn user_collection_name() -> &'static str {
        "instagramUsers"
    }

    fn chat_collection_name() -> &'static str {
        "instagramChats"
    }

    fn content_collection_name() -> &'static str {
        "instagramData"
    }
}

#[async_trait]
impl DefaultDataFetcherInfo for InstagramAPI {
    type UserInfo = UserInfo;
    type Content = Post;

    fn data_fetcher(&self) -> &ApiDataFetcher {
        &self.data_fetcher
    }

    fn subscription_type_to_api_type(s: SubscriptionType) -> &'static str {
        match s {
            SubscriptionType::Subscription1 => "stories",
            SubscriptionType::Subscription2 => "posts",
        }
    }

    async fn get_content_id_from_url(&self, url: &str) -> Option<String> {
        regexp::INSTAGRAM_LINK
            .captures(url)
            .and_then(|cap| cap.get(2))
            .map(|m| m.as_str().to_string())
    }
}

/*
   Structures for JSON deserialization
*/

#[derive(Deserialize, Debug)]
struct Resource {
    #[serde(rename = "pk")]
    id: String,
    video_url: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Post {
    id: String,
    thumbnail_url: Option<String>,
    #[serde(rename = "code")]
    url_code: String,
    caption_text: Option<String>,
    product_type: String,
    user: UserInfo,
    video_url: Option<String>,
    resources: Option<Vec<Resource>>,
}

impl GetId for Post {
    fn id(&self) -> &str {
        &self.id
    }
}

impl ReturnDataForDownload for Post {
    fn is_data_for_download(&self) -> bool {
        true
    }

    fn data(&self) -> Vec<DataForDownload> {
        if self.resources.is_some() && !self.resources.as_ref().unwrap().is_empty() {
            self.resources
                .as_ref()
                .unwrap()
                .iter()
                .map(|r| {
                    let (url, data_type) = if let Some(video_url) = &r.video_url {
                        (video_url.clone(), DataType::Video)
                    } else {
                        (r.thumbnail_url.as_ref().unwrap().clone(), DataType::Image)
                    };
                    DataForDownload {
                        url,
                        name: r.id.clone(),
                        data_type,
                    }
                })
                .collect()
        } else {
            let (url, data_type) = if let Some(video_url) = &self.video_url {
                (video_url.clone(), DataType::Video)
            } else {
                (
                    self.thumbnail_url.as_ref().unwrap().clone(),
                    DataType::Image,
                )
            };
            vec![DataForDownload {
                url,
                name: self.id.clone(),
                data_type,
            }]
        }
    }
}

impl ReturnUserInfo for Post {
    fn id(&self) -> &str {
        self.user.id()
    }

    fn unique_user_name(&self) -> &str {
        self.user.unique_user_name()
    }

    fn nickname(&self) -> &str {
        self.user.nickname()
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct UserInfo {
    #[serde(rename = "pk")]
    id: String,
    username: String,
    full_name: Option<String>,
}

impl ReturnUserInfo for UserInfo {
    fn id(&self) -> &str {
        &self.id
    }

    fn unique_user_name(&self) -> &str {
        &self.username
    }

    fn nickname(&self) -> &str {
        if let Some(full_name) = &self.full_name {
            if full_name.is_empty() {
                &self.username
            } else {
                full_name
            }
        } else {
            &self.username
        }
    }
}

impl PrepareDescription<UserInfo, Post> for InstagramAPI {
    fn prepare_description(
        user_info: &UserInfo,
        content: &Post,
        stype: &OutputType,
    ) -> DescriptionBuilder {
        let post_url = || format!("https://instagram.com/p/{}/", content.url_code);
        let story_url = || {
            format!(
                "https://instagram.com/stories/{}/{}",
                user_info.unique_user_name(),
                content.id
            )
        };
        let user_link = |user: &str| format!("https://instagram.com/{}/", user);

        let mut builder = DescriptionBuilder::new();
        match stype {
            OutputType::BySubscription(stype) => match stype {
                SubscriptionType::Subscription1 => builder.content("story", &story_url()),
                SubscriptionType::Subscription2 => builder.content("post", &post_url()),
            }
            .action(ActionType::Posted)
            .who(user_info.nickname(), &user_link(&user_info.username)),

            OutputType::ByLink(tguser) => if content.product_type == "story" {
                builder.content("story", &story_url())
            } else {
                builder.content("post", &post_url())
            }
            .who(&tguser.name, &tguser.user_link())
            .action(ActionType::Shared)
            .from(user_info.nickname(), &user_link(&user_info.username)),
        };

        if let Some(caption) = &content.caption_text {
            builder.description(caption.to_string());
        }
        builder
    }
}
