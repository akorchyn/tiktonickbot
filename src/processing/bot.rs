use teloxide::{prelude::*, utils::command::BotCommand};

use super::*;
use crate::api::{ApiContentReceiver, ApiUserInfoReceiver};
use crate::database::tiktok::DatabaseApi;

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "shows subscriptions for this chat")]
    ShowSubscriptions,
    #[command(description = "sends last like for given user.")]
    LastLike(String),
    #[command(
        description = "sends last n likes for given user.",
        parse_with = "split"
    )]
    LastNLike { username: String, n: u8 },
    #[command(description = "sends last video for given user.")]
    LastVideo(String),
    #[command(
        description = "sends last n videos for given user.",
        parse_with = "split"
    )]
    LastNVideo { username: String, n: u8 },
    #[command(description = "subscribe chat to tiktok user likes feed.")]
    SubscribeLikes(String),
    #[command(description = "subscribe chat to tiktok user likes feed.")]
    SubscribeVideo(String),
    #[command(description = "unsubscribe chat from tiktok user video feed.")]
    UnsubscribeLikes(String),
    #[command(description = "unsubscribe chat from tiktok user video feed.")]
    UnsubscribeVideo(String),
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
        Command::LastLike(username) => {
            last_n_videos(&cx, username, 1, tiktokapi::SubscriptionType::Likes).await
        }
        Command::LastNLike { username, n } => {
            last_n_videos(&cx, username, n, tiktokapi::SubscriptionType::Likes).await
        }
        Command::LastVideo(username) => {
            last_n_videos(&cx, username, 1, tiktokapi::SubscriptionType::CreatedVideos).await
        }
        Command::LastNVideo { username, n } => {
            last_n_videos(&cx, username, n, tiktokapi::SubscriptionType::CreatedVideos).await
        }
        Command::SubscribeLikes(username) => {
            subscribe(&cx, username, &chat_id, tiktokapi::SubscriptionType::Likes).await
        }
        Command::SubscribeVideo(username) => {
            subscribe(
                &cx,
                username,
                &chat_id,
                tiktokapi::SubscriptionType::CreatedVideos,
            )
            .await
        }
        Command::UnsubscribeLikes(username) => {
            unsubscribe(&cx, username, &chat_id, tiktokapi::SubscriptionType::Likes).await
        }
        Command::UnsubscribeVideo(username) => {
            unsubscribe(
                &cx,
                username,
                &chat_id,
                tiktokapi::SubscriptionType::CreatedVideos,
            )
            .await
        }
    };
    log::info!("Command handling finished");
    if let Err(e) = &status {
        if let Err(_) = cx.answer("Unfortunately, you request failed. Please, check input data corectness. If you are sure that your input is correct. Try again later").await {
            log::info!("Failed to respond to user with error message.\nInitial error: {}", e.to_string());
        }
    }
    status
}

async fn show_subscriptions(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    chat_id: &str,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;

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

async fn last_n_videos(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    n: u8,
    etype: tiktokapi::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let api = tiktokapi::TiktokApi::from_env();
    let user_info: tiktokapi::UserInfo = api.get_user_info(&username).await?;
    let liked_videos = api.get_content(&username, n, etype).await?;
    let mut succeed = true;
    download_videos(&liked_videos).await;
    for video in liked_videos {
        send_video(
            &cx.requester,
            &user_info,
            &cx.update.chat.id.to_string(),
            &video,
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
    stype: tiktokdb::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;

    if db.is_user_subscribed(&username, chat_id, stype).await? {
        cx.answer(format!("You already subscribed to {}", &username))
            .await?;
        Ok(())
    } else {
        let api = tiktokapi::TiktokApi::from_env();
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
    stype: tiktokdb::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;

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
