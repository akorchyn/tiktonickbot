use teloxide::{prelude::*, utils::command::BotCommand};

use crate::api::tiktok::{SubscriptionType as TiktokSub, TiktokApi};
use crate::api::twitter::{SubscriptionType as TwitterSub, TwitterApi};
use crate::api::*;
use crate::database::tiktok::DatabaseApi;

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
    SubscribeLikes(String),
    #[command(
        rename = "sub_tiktok",
        description = "subscribe chat to tiktok user likes feed."
    )]
    SubscribeVideo(String),
    #[command(
        rename = "unsub_tiktok_likes",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    UnsubscribeLikes(String),
    #[command(
        rename = "unsub_tiktok_likes",
        description = "unsubscribe chat from tiktok user video feed."
    )]
    UnsubscribeVideo(String),
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
            last_n_data::<TwitterApi>(&cx, username, 1, TwitterSub::Tweets).await
        }
        Command::LastNTweets { username, n } => {
            last_n_data::<TwitterApi>(&cx, username, n, TwitterSub::Tweets).await
        }
        Command::LastLikedTweet(username) => {
            last_n_data::<TwitterApi>(&cx, username, 1, TwitterSub::Likes).await
        }
        Command::LastNLikedTweet { username, n } => {
            last_n_data::<TwitterApi>(&cx, username, n, TwitterSub::Likes).await
        }
        Command::LastLike(username) => {
            last_n_data::<TiktokApi>(&cx, username, 1, TiktokSub::Likes).await
        }
        Command::LastNLike { username, n } => {
            last_n_data::<TiktokApi>(&cx, username, n, TiktokSub::Likes).await
        }
        Command::LastVideo(username) => {
            last_n_data::<TiktokApi>(&cx, username, 1, TiktokSub::CreatedVideos).await
        }
        Command::LastNVideo { username, n } => {
            last_n_data::<TiktokApi>(&cx, username, n, TiktokSub::CreatedVideos).await
        }
        Command::SubscribeLikes(username) => {
            subscribe(&cx, username, &chat_id, TiktokSub::Likes).await
        }
        Command::SubscribeVideo(username) => {
            subscribe(&cx, username, &chat_id, TiktokSub::CreatedVideos).await
        }
        Command::UnsubscribeLikes(username) => {
            unsubscribe(&cx, username, &chat_id, TiktokSub::Likes).await
        }
        Command::UnsubscribeVideo(username) => {
            unsubscribe(&cx, username, &chat_id, TiktokSub::CreatedVideos).await
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

async fn show_subscriptions(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    chat_id: &str,
) -> Result<(), anyhow::Error> {
    let db = super::create_db().await?;

    let chat = db.get_chat_info(chat_id).await?;
    let empty_text = "Currently, group doesn't have any subscriptions";
    if let Some(chat) = chat {
        let content = chat.subscribed_for_content_to.unwrap_or(Vec::new());
        let likes = chat.subscribed_for_likes_to.unwrap_or(Vec::new());
        if !content.is_empty() || !likes.is_empty() {
            let content_subscribers = content.into_iter().fold(String::new(), |result, i| {
                result + &format!("@{} - Content-type subscription\n", i)
            });
            let like_subscribers = likes.into_iter().fold(String::new(), |result, i| {
                result + &format!("@{} - Like-type subscription\n", i)
            });
            cx.answer(format!(
                "Group subscriptions:\n{}{}",
                content_subscribers, like_subscribers
            ))
            .await?;
        } else {
            cx.answer(empty_text).await?;
        }
    } else {
        cx.answer(empty_text).await?;
    }
    Ok(())
}

async fn last_n_data<Api>(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    n: u8,
    etype: Api::ContentType,
) -> Result<(), anyhow::Error>
where
    Api: ApiContentReceiver + ApiUserInfoReceiver + FromEnv<Api>,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    <Api as ApiContentReceiver>::ContentType: Copy
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
{
    let api = Api::from_env();
    let user_info = api.get_user_info(&username).await?;
    let mut data = api.get_content(&user_info.id(), n, etype).await?;
    data.truncate(n as usize);
    let mut succeed = true;
    super::download_content(&data).await;
    for i in data.into_iter().rev() {
        super::send_content(
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

async fn subscribe(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    chat_id: &str,
    stype: TiktokSub,
) -> Result<(), anyhow::Error> {
    let db = super::create_db().await?;

    if db.is_user_subscribed(&username, chat_id, stype).await? {
        cx.answer(format!("You already subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        let api = TiktokApi::from_env();
        let user_info = api.get_user_info(&username).await?;
        let likes = api.get_content(&username, 5, stype).await?;
        for like in likes {
            db.add_content(&like.id, &user_info.unique_user_id, stype)
                .await?;
        }
        db.subscribe_user(&username, &chat_id, stype).await?;
        cx.answer(format!(
            "Successfully subscribed to {} aka {}",
            &username, &user_info.nickname
        ))
        .await?;
        Ok(())
    }
}

async fn unsubscribe(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    chat_id: &str,
    stype: TiktokSub,
) -> Result<(), anyhow::Error> {
    let db = super::create_db().await?;

    if !db.is_user_subscribed(&username, chat_id, stype).await? {
        cx.answer(format!("You were not subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        db.unsubscribe_user(&username, &chat_id, stype).await?;
        cx.answer(format!("Successfully unsubscribed from {}", username))
            .await?;
        Ok(())
    }
}
