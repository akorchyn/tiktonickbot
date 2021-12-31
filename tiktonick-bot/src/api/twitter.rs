use reqwest::{self, Client};

use std::env;

use crate::api::{
    Api, ApiAlive, ApiContentReceiver, ApiName, ApiUserInfoReceiver, DataForDownload, DataType,
    DatabaseInfoProvider, FromEnv, GenerateSubscriptionMessage, GetId, ReturnDataForDownload,
    ReturnTextInfo, ReturnUserInfo, SubscriptionType,
};
use anyhow;
use anyhow::Error;
use async_trait::async_trait;
use serde::{self, Deserialize};
use serde_json;
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
    fn username(&self) -> &str {
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
    async fn get_data<T>(&self, uri: String) -> Result<Option<T>, anyhow::Error>
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
            Ok(None)
        }
    }

    fn subscription_type_to_api_uri(
        stype: SubscriptionType,
        user_id: &str,
        max_results: u8,
    ) -> String {
        static TWEET_FIELDS: &str =
            "id,text,attachments,author_id,in_reply_to_user_id,referenced_tweets,source";
        static EXPANSIONS: &str = "author_id,attachments.media_keys";
        static MEDIA_FIELDS: &str = "preview_image_url,url,media_key";
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

impl GenerateSubscriptionMessage<UserInfo, Tweet> for TwitterApi {
    fn subscription_message(user: &UserInfo, tweet: &Tweet, stype: SubscriptionType) -> String {
        let tweet_link = format!("https://twitter.com/{}/status/{}", tweet.username, tweet.id);
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

    fn subscription_format() -> Option<ParseMode> {
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
            .get_data::<Tweets>(TwitterApi::subscription_type_to_api_uri(stype, id, count))
            .await?;
        if let Some(tweets) = tweets {
            let media = tweets.includes.media.unwrap_or(Vec::new());

            Ok(tweets
                .data
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
        } else {
            Ok(Vec::new())
        }
    }
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

#[derive(Debug, Deserialize, Default)]
struct Tweets {
    data: Vec<TweetRespond>,
    includes: Includes,
}

#[derive(Debug, Deserialize, Default)]
struct UserApiResponse {
    data: UserInfo,
}
