use std::sync::mpsc::SyncSender;
use teloxide::{prelude2::*, utils::command::BotCommand};

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use crate::api::*;
use crate::database::MongoDatabase;
use crate::processing::{LinkInfo, RequestModel, UserRequest};
use crate::regexp;

#[derive(BotCommand, Clone)]
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
}

#[derive(Clone)]
struct ConfigParameters {
    req_sender: SyncSender<UserRequest>,
}

pub(crate) async fn run(bot: AutoSend<Bot>, req_sender: SyncSender<UserRequest>) {
    let parameters = ConfigParameters { req_sender };

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(command_handling),
        )
        .branch(
            dptree::filter(|msg: Message| {
                let text = msg.text().unwrap_or_default();
                regexp::TWITTER_LINK.is_match(text)
                    || regexp::TIKTOK_FULL_LINK.is_match(text)
                    || regexp::TIKTOK_SHORT_LINK.is_match(text)
            })
            .endpoint(link_handler),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![parameters])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

async fn link_handler(
    message: Message,
    _: AutoSend<Bot>,
    cfg: ConfigParameters,
) -> anyhow::Result<()> {
    let chat_id = message.chat_id().to_string();
    if message.from().is_none() {
        return Ok(());
    }
    let from = message.from().unwrap();
    let tguser = TelegramUser {
        id: from.id,
        name: from.first_name.clone() + " " + from.last_name.as_ref().unwrap_or(&String::new()),
    };
    let text = message.text().unwrap_or_default();
    if text.is_empty() {
        return Ok(());
    }
    for (matches, api) in vec![
        (regexp::TWITTER_LINK.find_iter(text), Api::Twitter),
        (regexp::TIKTOK_FULL_LINK.find_iter(text), Api::Tiktok),
        (regexp::TIKTOK_SHORT_LINK.find_iter(text), Api::Tiktok),
    ] {
        for m in matches {
            let link_info = LinkInfo {
                chat_id: chat_id.clone(),
                telegram_user: tguser.clone(),
                api,
                link: m.as_str().to_string(),
            };
            log::info!("found {}", &link_info.link);
            cfg.req_sender.send(UserRequest::ProcessLink(link_info))?;
        }
    }
    Ok(())
}

async fn command_handling(
    message: Message,
    bot: AutoSend<Bot>,
    command: Command,
    cfg: ConfigParameters,
) -> anyhow::Result<()> {
    let chat_id = message.chat.id.to_string();
    let status = match command {
        Command::Help => {
            bot.send_message(message.chat_id(), Command::descriptions())
                .await?;
            Ok(())
        }
        Command::ShowSubscriptions => show_subscriptions(&bot, &chat_id).await,
        Command::LastTweet(username) => {
            last_n_data::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                1,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::LastNTweets { username, n } => {
            last_n_data::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                n,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::LastLikedTweet(username) => {
            last_n_data::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                1,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::LastNLikedTweet { username, n } => {
            last_n_data::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                n,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::LastLike(username) => {
            last_n_data::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                1,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::LastNLike { username, n } => {
            last_n_data::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                n,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::LastVideo(username) => {
            last_n_data::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                1,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::LastNVideo { username, n } => {
            last_n_data::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                n,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::TiktokSubscribeLikes(username) => {
            subscribe::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::TiktokSubscribeVideo(username) => {
            subscribe::<TiktokApi>(
                &bot,
                &cfg.req_sender,
                username,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::TiktokUnsubscribeLikes(username) => {
            unsubscribe::<TiktokApi>(&bot, username, &chat_id, SubscriptionType::Likes).await
        }
        Command::TiktokUnsubscribeVideo(username) => {
            unsubscribe::<TiktokApi>(&bot, username, &chat_id, SubscriptionType::Content).await
        }
        Command::TwitterSubscribeLikes(username) => {
            subscribe::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                &chat_id,
                SubscriptionType::Likes,
            )
            .await
        }
        Command::TwitterSubscribeVideo(username) => {
            subscribe::<TwitterApi>(
                &bot,
                &cfg.req_sender,
                username,
                &chat_id,
                SubscriptionType::Content,
            )
            .await
        }
        Command::TwitterUnsubscribeLikes(username) => {
            unsubscribe::<TwitterApi>(&bot, username, &chat_id, SubscriptionType::Likes).await
        }
        Command::TwitterUnsubscribeVideo(username) => {
            unsubscribe::<TwitterApi>(&bot, username, &chat_id, SubscriptionType::Content).await
        }
    };
    log::info!("Command handling finished");
    if let Err(e) = &status {
        let error_message = "Unfortunately, you request failed. Please, check input data correctness. If you are sure that your input is correct. Try again later";
        if bot
            .send_message(message.chat.id, error_message)
            .await
            .is_err()
        {
            log::error!(
                "Failed to respond to user with error message.\nInitial error: {}",
                e.to_string()
            );
        }
    }
    Ok(())
}

async fn get_subscription_string_for_api<Api: DatabaseInfoProvider + ApiName>(
    chat_id: &str,
    db: &MongoDatabase,
) -> Result<String, anyhow::Error> {
    let chat = db.get_chat_info::<Api>(chat_id).await?;
    if let Some(chat) = chat {
        let content = chat.subscribed_for_content_to.unwrap_or_default();
        let likes = chat.subscribed_for_likes_to.unwrap_or_default();
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

async fn show_subscriptions(bot: &AutoSend<Bot>, chat_id: &str) -> anyhow::Result<()> {
    let db = super::create_db().await?;
    let tiktok_subs = get_subscription_string_for_api::<TiktokApi>(chat_id, &db)
        .await
        .unwrap_or_default();
    let twitter_subs = get_subscription_string_for_api::<TwitterApi>(chat_id, &db)
        .await
        .unwrap_or_default();

    let empty_text = "Currently, group doesn't have any subscriptions";
    if !tiktok_subs.is_empty() || !twitter_subs.is_empty() {
        bot.send_message(
            chat_id.parse::<i64>().unwrap(),
            format!("Group subscriptions:\n{}{}", tiktok_subs, twitter_subs),
        )
        .await?;
    } else {
        bot.send_message(chat_id.parse::<i64>().unwrap(), empty_text)
            .await?;
    }
    Ok(())
}

async fn last_n_data<Api>(
    bot: &AutoSend<Bot>,
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
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
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
    bot.send_message(
        chat_id.parse::<i64>()?,
        "Command added to the queue. You will be notified once its processed",
    )
    .await?;
    Ok(())
}

async fn subscribe<Api>(
    bot: &AutoSend<Bot>,
    req_sender: &SyncSender<UserRequest>,
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider + ApiName,
{
    let db = super::create_db().await?;
    let id: i64 = chat_id.parse()?;

    if db
        .is_user_subscribed::<Api>(&username, chat_id, stype)
        .await?
    {
        bot.send_message(id, format!("You already subscribed to {}", &username))
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
        bot.send_message(
            id,
            "Added to the queue. You will be notified once we subscribe".to_string(),
        )
        .await?;
        Ok(())
    }
}

async fn unsubscribe<Api>(
    cx: &AutoSend<Bot>,
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider,
{
    let db = super::create_db().await?;
    let id: i64 = chat_id.parse()?;
    if !db
        .is_user_subscribed::<Api>(&username, chat_id, stype)
        .await?
    {
        cx.send_message(id, format!("You were not subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        db.unsubscribe_user::<Api>(&username, chat_id, stype)
            .await?;
        cx.send_message(id, format!("Successfully unsubscribed from {}", &username))
            .await?;
        Ok(())
    }
}
