use futures::{future, Future, FutureExt};
use std::fmt::Debug;
use std::sync::mpsc::SyncSender;
use teloxide::types::Me;
use teloxide::{prelude::*, utils::command::BotCommand};

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use crate::api::*;
use crate::database::MongoDatabase;
use crate::processing::{RequestModel, UserRequest};

use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    // General info:
    #[command(description = "display this text.")]
    Help,
    #[command(
        rename = "subscriptions",
        description = "shows subscriptions for this chat"
    )]
    ShowSubscriptions,
    // Twitter commands:
    #[command(rename = "tweet", description = "sends last tweet for given user.")]
    LastTweet(String),
    #[command(
        rename = "tweets",
        description = "sends last n tweets for given user.",
        parse_with = "split"
    )]
    LastNTweets { username: String, n: u8 },
    #[command(
        rename = "ltweet",
        description = "sends last liked tweet for given user."
    )]
    LastLikedTweet(String),
    #[command(
        rename = "ltweets",
        description = "sends last n liked tweets for given user.",
        parse_with = "split"
    )]
    LastNLikedTweet { username: String, n: u8 },

    // Tiktok commands:
    #[command(rename = "ltiktok", description = "sends last like for given user.")]
    LastLike(String),
    #[command(
        rename = "ltiktoks",
        description = "sends last n likes for given user.",
        parse_with = "split"
    )]
    LastNLike { username: String, n: u8 },
    #[command(rename = "tiktok", description = "sends last video for given user.")]
    LastVideo(String),
    #[command(
        rename = "tiktoks",
        description = "sends last n videos for given user.",
        parse_with = "split"
    )]
    LastNVideo { username: String, n: u8 },
    #[command(
        rename = "sub_tiktok_likes",
        description = "subscribe chat to tiktok user likes feed."
    )]
    TiktokSubscribeLikes(String),
    #[command(
        rename = "sub_tiktok",
        description = "subscribe chat to tiktok user likes feed."
    )]
    TiktokSubscribeVideo(String),
    #[command(
        rename = "unsub_tiktok_likes",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    TiktokUnsubscribeLikes(String),
    #[command(
        rename = "unsub_tiktok",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    TiktokUnsubscribeVideo(String),
    #[command(
        rename = "sub_twitter_likes",
        description = "subscribe chat to tiktok user likes feed."
    )]
    TwitterSubscribeLikes(String),
    #[command(
        rename = "sub_twitter",
        description = "subscribe chat to tiktok user likes feed."
    )]
    TwitterSubscribeVideo(String),
    #[command(
        rename = "unsub_twitter_likes",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    TwitterUnsubscribeLikes(String),
    #[command(
        rename = "unsub_twitter",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    TwitterUnsubscribeVideo(String),
    #[command(description = "off")]
    SetNewCookie(String),
}

pub(crate) async fn run(bot: AutoSend<Bot>, req_sender: SyncSender<UserRequest>) {
    let Me { user, .. } = bot.get_me().await.expect("Couldn't get myself :(");
    let name = user.username.expect("Bots *must* have usernames");

    let commands = |(cx, command): (UpdateWithCx<AutoSend<Bot>, Message>, Command),
                    req_sender: SyncSender<UserRequest>| async move {
        let message = &cx.update;
        let chat_id = message.chat.id.to_string();
        let status = match command {
            Command::Help => {
                cx.answer(Command::descriptions()).await?;
                Ok(())
            }
            Command::ShowSubscriptions => show_subscriptions(&cx, &chat_id).await,
            Command::LastTweet(username) => {
                last_n_data::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    1,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::LastNTweets { username, n } => {
                last_n_data::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    n,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::LastLikedTweet(username) => {
                last_n_data::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    1,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::LastNLikedTweet { username, n } => {
                last_n_data::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    n,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::LastLike(username) => {
                last_n_data::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    1,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::LastNLike { username, n } => {
                last_n_data::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    n,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::LastVideo(username) => {
                last_n_data::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    1,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::LastNVideo { username, n } => {
                last_n_data::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    n,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::TiktokSubscribeLikes(username) => {
                subscribe::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::TiktokSubscribeVideo(username) => {
                subscribe::<TiktokApi>(
                    &cx,
                    &req_sender,
                    username,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::TiktokUnsubscribeLikes(username) => {
                unsubscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Likes).await
            }
            Command::TiktokUnsubscribeVideo(username) => {
                unsubscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Content).await
            }
            Command::TwitterSubscribeLikes(username) => {
                subscribe::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    &chat_id,
                    SubscriptionType::Likes,
                )
                .await
            }
            Command::TwitterSubscribeVideo(username) => {
                subscribe::<TwitterApi>(
                    &cx,
                    &req_sender,
                    username,
                    &chat_id,
                    SubscriptionType::Content,
                )
                .await
            }
            Command::TwitterUnsubscribeLikes(username) => {
                unsubscribe::<TwitterApi>(&cx, username, &chat_id, SubscriptionType::Likes).await
            }
            Command::TwitterUnsubscribeVideo(username) => {
                unsubscribe::<TwitterApi>(&cx, username, &chat_id, SubscriptionType::Content).await
            }
            Command::SetNewCookie(cookie) => {
                log::info!("Sending new cookie to the api: {}", cookie);
                if let Some(user) = cx.update.from() {
                    let admin_id: String = std::env::var("TELEGRAM_ADMIN_ID").unwrap();
                    if user.id.to_string() == admin_id {
                        TiktokApi::from_env().send_api_new_cookie(cookie).await?;
                        cx.answer("Succeed").await?;
                    } else {
                        cx.answer("Not authorized").await?;
                    }
                }
                Ok(())
            }
        };
        log::info!("Command handling finished");
        if let Err(e) = &status {
            if let Err(_) = cx.reply_to("Unfortunately, you request failed. Please, check input data correctness. If you are sure that your input is correct. Try again later").await {
                log::info!("Failed to respond to user with error message.\nInitial error: {}", e.to_string());
            }
        }
        status
    };

    let mut dp = Dispatcher::new(bot)
        .messages_handler(move |rx| async move {
            UnboundedReceiverStream::new(rx)
                .commands(name)
                .for_each_concurrent(None, err(with(req_sender, commands)))
                .await
        })
        .setup_ctrlc_handler();
    dp.dispatch().await;
}

/// Process errors (log)
fn err<T, E, F>(f: impl Fn(T) -> F) -> impl Fn(T) -> future::Map<F, fn(Result<(), E>) -> ()>
where
    F: Future<Output = Result<(), E>>,
    E: Debug,
{
    move |x| {
        f(x).map(|r| {
            if let Err(err) = r {
                log::error!("Error in handler: {:?}", err);
            }
        })
    }
}

fn with<A, B, U>(ctx: B, f: impl Fn(A, B) -> U) -> impl Fn(A) -> U
where
    B: Clone,
{
    move |a| f(a, ctx.clone())
}

async fn get_subscription_string_for_api<Api: DatabaseInfoProvider + ApiName>(
    chat_id: &str,
    db: &MongoDatabase,
) -> Result<String, anyhow::Error> {
    let chat = db.get_chat_info::<Api>(chat_id).await?;
    if let Some(chat) = chat {
        let content = chat.subscribed_for_content_to.unwrap_or(Vec::new());
        let likes = chat.subscribed_for_likes_to.unwrap_or(Vec::new());
        let content_subscribers = content.into_iter().fold(String::new(), |result, i| {
            result + &format!("@{} - Content-type subscription\n", i)
        });
        let like_subscribers = likes.into_iter().fold(String::new(), |result, i| {
            result + &format!("@{} - Like-type subscription\n", i)
        });
        if !content_subscribers.is_empty() || !like_subscribers.is_empty() {
            Ok(format!(
                "{}:\n{}{}\n",
                Api::name(),
                content_subscribers,
                like_subscribers
            ))
        } else {
            Ok(String::new())
        }
    } else {
        Ok(String::new())
    }
}

async fn show_subscriptions(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    chat_id: &str,
) -> Result<(), anyhow::Error> {
    let db = super::create_db().await?;
    let tiktok_subs = get_subscription_string_for_api::<TiktokApi>(chat_id, &db)
        .await
        .unwrap_or(String::new());
    let twitter_subs = get_subscription_string_for_api::<TwitterApi>(chat_id, &db)
        .await
        .unwrap_or(String::new());

    let empty_text = "Currently, group doesn't have any subscriptions";
    if !tiktok_subs.is_empty() || !twitter_subs.is_empty() {
        cx.answer(format!(
            "Group subscriptions:\n{}{}",
            tiktok_subs, twitter_subs
        ))
        .await?;
    } else {
        cx.answer(empty_text).await?;
    }
    Ok(())
}

async fn last_n_data<Api>(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    request_sender: &SyncSender<UserRequest>,
    username: String,
    n: u8,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: ApiContentReceiver
        + ApiName
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    let model = RequestModel {
        chat_id: chat_id.to_string(),
        stype,
        user: username,
        api: Api::api_type(),
    };
    request_sender.send(UserRequest::LastNData(model, n))?;
    cx.reply_to("Command added to the queue. You will be notified once its processed")
        .await?;
    Ok(())
}

async fn subscribe<Api>(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    req_sender: &SyncSender<UserRequest>,
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider + ApiName,
{
    let db = super::create_db().await?;

    if db
        .is_user_subscribed::<Api>(&username, chat_id, stype)
        .await?
    {
        cx.answer(format!("You already subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        let model = RequestModel {
            chat_id: chat_id.to_string(),
            stype,
            user: username.to_string(),
            api: Api::api_type(),
        };
        req_sender.send(UserRequest::Subscribe(model))?;
        cx.reply_to(format!(
            "Added to the queue. You will be notified once we subscribe"
        ))
        .await?;
        Ok(())
    }
}

async fn unsubscribe<Api>(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider,
{
    let db = super::create_db().await?;
    if !db
        .is_user_subscribed::<Api>(&username, chat_id, stype)
        .await?
    {
        cx.answer(format!("You were not subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        db.unsubscribe_user::<Api>(&username, &chat_id, stype)
            .await?;
        cx.answer(format!("Successfully unsubscribed from {}", &username))
            .await?;
        Ok(())
    }
}
