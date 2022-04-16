use crate::api::{
    api_data_fetcher, Api, ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload,
    DataType, DatabaseInfoProvider, FromEnv, GenerateMessage, GetId, OutputType,
    ReturnDataForDownload, ReturnUserInfo, ReturnUsername, SubscriptionType,
};
use crate::regexp;

use crate::api::api_data_fetcher::Request;
use crate::common::description_builder::{ActionType, DescriptionBuilder};
use anyhow::anyhow;
use async_trait::async_trait;
use serde::{self, Deserialize};
use teloxide::types::ParseMode;
use teloxide::types::ParseMode::Html;

#[derive(Debug, Deserialize, Default)]
pub(crate) struct UserInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) username: String,
}

impl ReturnUserInfo for UserInfo {
    fn id(&self) -> &str {
        &self.id
    }
    fn unique_user_name(&self) -> &str {
        &self.username
    }
    fn nickname(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct Tweet {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) name: String,
    pub(crate) username: String,
    pub(crate) attachments: Option<Vec<DataForDownload>>,
}

impl ReturnUsername for Tweet {
    fn username(&self) -> &str {
        &self.username
    }
}

impl GetId for Tweet {
    fn id(&self) -> &str {
        &self.id
    }
}

impl ReturnDataForDownload for Tweet {
    fn is_data_for_download(&self) -> bool {
        self.attachments.is_some()
    }
    fn data(&self) -> Vec<super::DataForDownload> {
        if let Some(attachments) = &self.attachments {
            attachments.clone()
        } else {
            Vec::new()
        }
    }
}

pub(crate) struct TwitterAPI {
    pub(crate) data_fetcher: api_data_fetcher::ApiDataFetcher,
}

impl ApiName for TwitterAPI {
    fn name() -> &'static str {
        "Twitter"
    }
    fn api_type() -> Api {
        Api::Twitter
    }
}

impl GenerateMessage<UserInfo, Tweet> for TwitterAPI {
    fn message(user: &UserInfo, tweet: &Tweet, output: &OutputType, len: usize) -> String {
        let tweet_link = format!("https://twitter.com/{}/status/{}", tweet.username, tweet.id);
        let user_link = |user: &str| format!("https://twitter.com/{}", user);
        let mut builder = DescriptionBuilder::new();

        match output {
            OutputType::BySubscription(stype) => match stype {
                SubscriptionType::Subscription1 => builder
                    .action(ActionType::Liked)
                    .from(&tweet.name, &user_link(&tweet.username)),
                SubscriptionType::Subscription2 => builder.action(ActionType::Posted),
            }
            .who(&user.name, &user_link(&user.username)),
            OutputType::ByLink(tguser) => builder
                .who(&tguser.name, &tguser.user_link())
                .action(ActionType::Shared)
                .from(&tweet.name, &user_link(&tweet.username)),
        }
        .content("tweet", &tweet_link)
        .description(tweet.text.clone())
        .size_limit(len)
        .build()
    }

    fn message_format() -> ParseMode {
        Html
    }
}

impl DatabaseInfoProvider for TwitterAPI {
    fn user_collection_name() -> &'static str {
        "twitterUsers"
    }

    fn chat_collection_name() -> &'static str {
        "twitterChats"
    }

    fn content_collection_name() -> &'static str {
        "twitterData"
    }
}

impl FromEnv<TwitterAPI> for TwitterAPI {
    fn from_env() -> TwitterAPI {
        TwitterAPI {
            data_fetcher: api_data_fetcher::ApiDataFetcher::from_env(TwitterAPI::api_type()),
        }
    }
}

#[async_trait]
impl ApiContentReceiver for TwitterAPI {
    type Out = Tweet;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        stype: SubscriptionType,
    ) -> Result<Vec<Tweet>, anyhow::Error> {
        let api = match stype {
            SubscriptionType::Subscription2 => "posts",
            SubscriptionType::Subscription1 => "likes",
        };

        let count = count.min(100).max(5);
        let tweets = self
            .data_fetcher
            .get_data(Request::Content(id, api, count))
            .await?;
        if let Some(tweets) = tweets {
            Ok(process_tweet_data(tweets).await?)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Option<Tweet>> {
        if let Some(cap) = regexp::TWITTER_LINK.captures(link) {
            log::info!("Started processing request");
            let tweets = self
                .data_fetcher
                .get_data::<TwitterTweetResult>(Request::ContentById(&cap[3]))
                .await?;
            return if let Some(tweets) = tweets {
                log::info!("Started parsing received data");
                Ok(process_tweet_data(tweets).await?.pop())
            } else {
                Ok(None)
            };
        }
        return Err(anyhow!("Error processing {}", link));
    }
}

async fn process_tweet_data(tweets: TwitterTweetResult) -> Result<Vec<Tweet>, anyhow::Error> {
    let media = tweets.includes.media.unwrap_or_default();

    let data = match tweets.data {
        Data::Array(vec) => vec,
        Data::Single(elem) => vec![elem],
    };

    data.into_iter()
        .map(|tweet| -> anyhow::Result<Tweet> {
            let user_info = tweets
                .includes
                .users
                .iter()
                .find(|i| tweet.author_id == i.id)
                .ok_or_else(|| anyhow!("Twitter api failure. Api should provide user info"))?;
            let attachments = if tweet.attachments.is_some() && !media.is_empty() {
                let mut attachments = Vec::new();
                for media_key in tweet.attachments.unwrap().media_keys.into_iter() {
                    let elem = media.iter().find(|elem| elem.media_key == media_key);
                    // Unfortunately,currently twitter API v2 support not all media types
                    if let Some(elem) = elem {
                        let result = elem.url.as_ref().or(elem.preview_image_url.as_ref());
                        if let Some(url) = result {
                            attachments.push(DataForDownload {
                                url: url.clone(),
                                name: media_key,
                                data_type: DataType::Image,
                            });
                        }
                    }
                }
                Some(attachments)
            } else {
                None
            };
            Ok(Tweet {
                id: tweet.id,
                text: tweet.text,
                name: user_info.name.clone(),
                username: user_info.username.clone(),
                attachments,
            })
        })
        .collect::<anyhow::Result<Vec<Tweet>>>()
}

#[async_trait]
impl ApiUserInfoReceiver for TwitterAPI {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<Option<UserInfo>, anyhow::Error> {
        let user_info = self
            .data_fetcher
            .get_data::<UserApiResponse>(Request::UserData(id))
            .await;
        if let Ok(user_info) = user_info {
            Ok(user_info.map(|user_info| user_info.data))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct Media {
    media_key: String,
    url: Option<String>,
    preview_image_url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Includes {
    media: Option<Vec<Media>>,
    users: Vec<UserInfo>,
}

#[derive(Debug, Deserialize, Default)]
struct TweetAttachments {
    media_keys: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct TweetRespond {
    id: String,
    author_id: String,
    text: String,
    attachments: Option<TweetAttachments>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Data {
    Array(Vec<TweetRespond>),
    Single(TweetRespond),
}

#[derive(Debug, Deserialize)]
struct TwitterTweetResult {
    data: Data,
    includes: Includes,
}

#[derive(Debug, Deserialize, Default)]
struct UserApiResponse {
    data: UserInfo,
}
