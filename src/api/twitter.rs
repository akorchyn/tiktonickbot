use reqwest::{self, Client};

use std::env;

use crate::api::{
    ApiContentReceiver, ApiUserInfoReceiver, DataForDownload, DataType, FromEnv,
    GenerateSubscriptionMessage, ReturnDataForDownload, ReturnTextInfo, ReturnUserInfo,
};
use anyhow;
use async_trait::async_trait;
use serde::{self, Deserialize};
use serde_json;

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
    pub(crate) author_id: String,
    pub(crate) text: String,
    pub(crate) name: String,
    pub(crate) username: String,
    pub(crate) attachments: Option<Vec<DataForDownload>>,
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum SubscriptionType {
    Likes,
    Tweets,
}

impl SubscriptionType {
    pub(crate) fn iterator() -> impl Iterator<Item = SubscriptionType> {
        [SubscriptionType::Tweets, SubscriptionType::Likes]
            .iter()
            .copied()
    }

    fn to_api_uri(&self, user_id: &str, max_results: u8) -> String {
        static TWEET_FIELDS: &str =
            "id,text,attachments,author_id,in_reply_to_user_id,referenced_tweets,source";
        static EXPANSIONS: &str = "author_id,attachments.media_keys";
        static MEDIA_FIELDS: &str = "preview_image_url,url,media_key";
        let api = match *self {
            SubscriptionType::Tweets => "tweets",
            SubscriptionType::Likes => "liked_tweets",
        };
        format!(
            "2/users/{}/{}?tweet.fields={}&max_results={}&expansions={}&media.fields={}",
            user_id, api, &TWEET_FIELDS, max_results, EXPANSIONS, MEDIA_FIELDS
        )
    }
}

impl GenerateSubscriptionMessage<UserInfo, Tweet> for SubscriptionType {
    fn subscription_message(&self, user: &UserInfo, tweet: &Tweet) -> String {
        match *self {
            SubscriptionType::Likes => format!(
                "User {} aka {} liked tweet from {} aka {}.\n\nTweet:\n{}",
                user.username, user.name, tweet.username, tweet.name, tweet.text
            ),
            SubscriptionType::Tweets => format!(
                "User {} aka {} tweet:\n{}",
                tweet.username, tweet.name, tweet.text
            ),
        }
    }
}

pub(crate) struct TwitterApi {
    secret: String,
    twitter_domain: String,
}

impl TwitterApi {
    async fn get_data<T>(&self, uri: String) -> Result<T, anyhow::Error>
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
        let data = serde_json::from_str::<T>(&text)?;
        Ok(data)
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
    type ContentType = SubscriptionType;
    async fn get_content(
        &self,
        id: &str,
        count: u8,
        etype: SubscriptionType,
    ) -> Result<Vec<Tweet>, anyhow::Error> {
        let count = count.min(100).max(5);
        let tweets = self.get_data::<Tweets>(etype.to_api_uri(id, count)).await?;
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
                    author_id: tweet.author_id,
                    text: tweet.text,
                    name: user_info.name.clone(),
                    username: user_info.username.clone(),
                    attachments,
                }
            })
            .collect())
    }
}

#[async_trait]
impl ApiUserInfoReceiver for TwitterApi {
    type Out = UserInfo;
    async fn get_user_info(&self, id: &str) -> Result<UserInfo, anyhow::Error> {
        let user_info = self
            .get_data::<UserApiResponse>(format!("2/users/by/username/{}", id))
            .await?;
        Ok(user_info.data)
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
