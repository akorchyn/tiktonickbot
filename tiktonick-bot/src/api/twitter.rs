use reqwest::{self, Client};

use std::env;

use crate::api::{
    Api, ApiAlive, ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload, DataType,
    DatabaseInfoProvider, FromEnv, GenerateMessage, GetId, OutputType, ReturnDataForDownload,
    ReturnTextInfo, ReturnUserInfo, ReturnUsername, SubscriptionType,
};
use crate::regexp;

use anyhow::{anyhow, Error};
use async_trait::async_trait;
use serde::{self, Deserialize};
use serde_json;
use teloxide::types::ParseMode;
use teloxide::types::ParseMode::Html;

static TWEET_FIELDS: &str =
    "id,text,attachments,author_id,in_reply_to_user_id,referenced_tweets,source";
static EXPANSIONS: &str = "author_id,attachments.media_keys";
static MEDIA_FIELDS: &str = "preview_image_url,url,media_key";

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

impl ReturnTextInfo for Tweet {
    fn text_info(&self) -> &str {
        &self.text
    }
}

pub(crate) struct TwitterApi {
    secret: String,
    twitter_domain: String,
}

impl TwitterApi {
    async fn get_data<T>(&self, uri: String) -> anyhow::Result<Option<T>>
    where
        for<'a> T: Deserialize<'a>,
    {
        let client = Client::new();
        let response = client
            .get(format!("{}/{}", &self.twitter_domain, uri))
            .bearer_auth(&self.secret)
            .send()
            .await?;
        let text = response.text().await?;
        let data = serde_json::from_str::<T>(&text);
        if let Ok(data) = data {
            Ok(Some(data))
        } else {
            log::warn!("{}", data.err().unwrap());
            Ok(None)
        }
    }

    fn subscription_type_to_api_uri(
        stype: SubscriptionType,
        user_id: &str,
        max_results: u8,
    ) -> String {
        let api = match stype {
            SubscriptionType::Content => "tweets",
            SubscriptionType::Likes => "liked_tweets",
        };
        format!(
            "2/users/{}/{}?tweet.fields={}&max_results={}&expansions={}&media.fields={}",
            user_id, api, &TWEET_FIELDS, max_results, EXPANSIONS, MEDIA_FIELDS
        )
    }
}

#[async_trait]
impl ApiAlive for TwitterApi {
    async fn is_alive(&self) -> bool {
        // Twitter API is official. So we can just ignore for now. Probably, we will implement credentials switch in nearby future.
        true
    }

    async fn try_make_alive(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl ApiName for TwitterApi {
    fn name() -> &'static str {
        "Twitter"
    }
    fn api_type() -> Api {
        Api::Twitter
    }
}

impl GenerateMessage<UserInfo, Tweet> for TwitterApi {
    fn message(user: &UserInfo, tweet: &Tweet, output: &OutputType) -> String {
        let tweet_link = format!("https://twitter.com/{}/status/{}", tweet.username, tweet.id);
        match output {
            OutputType::BySubscription(stype) => {
                match stype {
                    SubscriptionType::Likes => format!(
                        "<i><a href=\"https://www.twitter.com/{}\">{}</a> liked <a href=\"{}\">tweet</a> from <a href=\"https://www.twitter.com/{}\">{}</a>:</i>\n\n{}",
                        user.username, user.name, tweet_link, tweet.username, tweet.name, tweet.text
                    ),
                    SubscriptionType::Content => format!(
                        "<i><a href=\"https://www.twitter.com/{}\">{}</a> posted <a href=\"{}\">tweet</a>:</i>\n\n{}",
                        tweet.username, tweet.name, tweet_link, tweet.text
                    ),
                }
            }
        OutputType::ByLink(tguser) => {
                format!("<i><a href=\"tg://user?id={}\">{}</a> shared <a href=\"{}\">tweet</a>:</i>\n\n{}",
                    tguser.id, tguser.name, tweet_link, tweet.text)
            }
        }
    }

    fn message_format() -> Option<ParseMode> {
        Some(Html)
    }
}

impl DatabaseInfoProvider for TwitterApi {
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

impl FromEnv<TwitterApi> for TwitterApi {
    fn from_env() -> TwitterApi {
        TwitterApi {
            secret: env::var("TWITTER_API_BEARER_SECRET").unwrap_or("blahblah".to_string()),
            twitter_domain: env::var("TWITTER_API_URL").unwrap_or("localhost:3000".to_string()),
        }
    }
}

#[async_trait]
impl ApiContentReceiver for TwitterApi {
    type Out = Tweet;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        stype: SubscriptionType,
    ) -> Result<Vec<Tweet>, anyhow::Error> {
        let count = count.min(100).max(5);
        let tweets = self
            .get_data::<TwitterTweetResult>(TwitterApi::subscription_type_to_api_uri(
                stype, id, count,
            ))
            .await?;
        if let Some(tweets) = tweets {
            Ok(process_tweet_data(tweets).await?)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_content_for_link(&self, link: &str) -> anyhow::Result<Tweet> {
        for cap in regexp::TWITTER_LINK.captures(link) {
            log::info!("Started processing request");
            let tweets = self
                .get_data::<TwitterTweetResult>(format!(
                    "2/tweets/{}?tweet.fields={}&expansions={}&media.fields={}",
                    &cap[3], TWEET_FIELDS, EXPANSIONS, MEDIA_FIELDS
                ))
                .await?;
            if let Some(tweets) = tweets {
                log::info!("Started parsing received data");
                if let Some(tweet) = process_tweet_data(tweets).await?.pop() {
                    return Ok(tweet);
                }
            }
        }
        return Err(anyhow!("Error processing {}", link));
    }
}

async fn process_tweet_data(tweets: TwitterTweetResult) -> Result<Vec<Tweet>, anyhow::Error> {
    let media = tweets.includes.media.unwrap_or(Vec::new());

    let data = match tweets.data {
        Data::Array(vec) => vec,
        Data::Single(elem) => vec![elem],
    };

    Ok(data
        .into_iter()
        .map(|tweet| {
            let user_info = tweets
                .includes
                .users
                .iter()
                .find(|i| tweet.author_id == i.id)
                .expect("Api should provide users info");
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
            Tweet {
                id: tweet.id,
                text: tweet.text,
                name: user_info.name.clone(),
                username: user_info.username.clone(),
                attachments,
            }
        })
        .collect())
}

#[async_trait]
impl ApiUserInfoReceiver for TwitterApi {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<Option<UserInfo>, anyhow::Error> {
        let user_info = self
            .get_data::<UserApiResponse>(format!("2/users/by/username/{}", id))
            .await?;
        Ok(user_info.and_then(|user_info| Some(user_info.data)))
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
