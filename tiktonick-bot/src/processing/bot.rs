use teloxide::{prelude::*, utils::command::BotCommand};

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use crate::api::*;
use crate::database::MongoDatabase;

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
        rename = "ltiktoks",
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
        rename = "unsub_tiktok_likes",
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
        rename = "unsub_twitter_likes",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    TwitterUnsubscribeVideo(String),
    #[command(description = "off")]
    SetNewCookie(String),
}

pub(crate) async fn run(bot: AutoSend<Bot>, bot_name: String) {
    teloxide::commands_repl(bot, bot_name, answer).await;
}

async fn answer(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    command: Command,
) -> Result<(), anyhow::Error> {
    let message = &cx.update;
    let chat_id = message.chat.id.to_string();
    let status = match command {
        Command::Help => {
            cx.answer(Command::descriptions()).await?;
            Ok(())
        }
        Command::ShowSubscriptions => show_subscriptions(&cx, &chat_id).await,
        Command::LastTweet(username) => {
            last_n_data::<TwitterApi>(&cx, username, 1, SubscriptionType::Content).await
        }
        Command::LastNTweets { username, n } => {
            last_n_data::<TwitterApi>(&cx, username, n, SubscriptionType::Content).await
        }
        Command::LastLikedTweet(username) => {
            last_n_data::<TwitterApi>(&cx, username, 1, SubscriptionType::Likes).await
        }
        Command::LastNLikedTweet { username, n } => {
            last_n_data::<TwitterApi>(&cx, username, n, SubscriptionType::Likes).await
        }
        Command::LastLike(username) => {
            last_n_data::<TiktokApi>(&cx, username, 1, SubscriptionType::Likes).await
        }
        Command::LastNLike { username, n } => {
            last_n_data::<TiktokApi>(&cx, username, n, SubscriptionType::Likes).await
        }
        Command::LastVideo(username) => {
            last_n_data::<TiktokApi>(&cx, username, 1, SubscriptionType::Content).await
        }
        Command::LastNVideo { username, n } => {
            last_n_data::<TiktokApi>(&cx, username, n, SubscriptionType::Content).await
        }
        Command::TiktokSubscribeLikes(username) => {
            subscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Likes).await
        }
        Command::TiktokSubscribeVideo(username) => {
            subscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Content).await
        }
        Command::TiktokUnsubscribeLikes(username) => {
            unsubscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Likes).await
        }
        Command::TiktokUnsubscribeVideo(username) => {
            unsubscribe::<TiktokApi>(&cx, username, &chat_id, SubscriptionType::Content).await
        }
        Command::TwitterSubscribeLikes(username) => {
            subscribe::<TwitterApi>(&cx, username, &chat_id, SubscriptionType::Likes).await
        }
        Command::TwitterSubscribeVideo(username) => {
            subscribe::<TwitterApi>(&cx, username, &chat_id, SubscriptionType::Content).await
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
            result + &format!("@{} - Like-type subscription", i)
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
    username: String,
    n: u8,
    etype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    let api = Api::from_env();
    let user_info = api.get_user_info(&username).await?;
    let mut data = api.get_content(&user_info.id(), n, etype).await?;
    data.truncate(n as usize);
    let mut succeed = true;
    super::download_content(&data).await;
    for i in data.into_iter().rev() {
        super::send_content(
            &api,
            &cx.requester,
            &user_info,
            &cx.update.chat.id.to_string(),
            &i,
            etype,
        )
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to send video with {}", e);
            succeed = false;
        });
    }
    if succeed {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to send some videos..."))
    }
}

async fn subscribe<Api>(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider + ApiContentReceiver + ApiUserInfoReceiver + FromEnv<Api>,
    <Api as ApiContentReceiver>::Out: GetId,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
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
        let api = Api::from_env();
        let user_info = api.get_user_info(&username).await?;
        let content = api.get_content(&user_info.id(), 5, stype).await?;
        for item in content {
            db.add_content::<Api>(&item.id(), &user_info.id(), stype)
                .await?;
        }
        db.subscribe_user::<Api>(&user_info.username(), &chat_id, stype)
            .await?;
        cx.answer(format!(
            "Successfully subscribed to {} aka {}",
            &username,
            &user_info.nickname()
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
