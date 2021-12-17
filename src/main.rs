use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;

use futures::future::join_all;
use futures::FutureExt;
use reqwest;
use teloxide::types::InputFile;
use teloxide::{prelude::*, utils::command::BotCommand};

use anyhow;
use database::MongoDatabase;

mod api;
mod database;

use crate::api::{ApiContentReceiver, ApiUserInfoReceiver};
use crate::tiktokapi::TiktokApi;
use crate::tiktokdb::{DatabaseApi, SubscriptionType};
use api::tiktok as tiktokapi;
use database::tiktok as tiktokdb;

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("TIKTOK_BOT_MONGO_CON_STRING") {
        if let Ok(database) = env::var("TIKTOK_BOT_DATABASE_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

async fn download_videos(liked_videos: &Vec<tiktokapi::Video>) {
    let futures: Vec<_> = liked_videos
        .into_iter()
        .map(|video| async {
            let filename = format!("videos/{}.mp4", video.id);
            if Path::new(&filename).exists() {
                log::info!(
                    "Video from user @{} is already cached({}). Skipping...",
                    video.unique_user_id,
                    filename
                );
                return true;
            }
            log::info!("Downloading video from user @{}", video.unique_user_id);
            reqwest::get(&video.download_address)
                .then(|response| async {
                    if let Ok(data) = response {
                        log::info!("Creating file {}", &filename);
                        if let Ok(mut file) = File::create(&filename) {
                            let bytes = data.bytes().await;
                            if let Ok(bytes) = bytes {
                                log::info!("Copying download data to file {}", &filename);
                                if let Ok(_) = io::copy(&mut bytes.as_ref(), &mut file) {
                                    return true;
                                }
                            }
                        }
                    }
                    false
                })
                .await
        })
        .collect();
    join_all(futures).await;
}

async fn last_n_videos(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    n: u8,
    etype: tiktokapi::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let api = TiktokApi::from_env();
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

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
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

async fn subscribe(
    username: String,
    chat_id: &str,
    stype: tiktokdb::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;
    let api = TiktokApi::from_env();
    let user_info = api.get_user_info(&username).await?;
    let likes = api.get_content(&username, 5, stype).await?;
    for like in likes {
        db.add_video(&like.id, &user_info.unique_user_id, stype)
            .await?;
    }
    db.subscribe_user(&username, &chat_id, stype).await
}

async fn unsubscribe(
    username: String,
    chat_id: &str,
    stype: tiktokdb::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;
    db.unsubscribe_user(&username, &chat_id, stype).await
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
        Command::LastLike(username) => {
            last_n_videos(cx, username, 1, tiktokapi::SubscriptionType::Likes).await
        }
        Command::LastNLike { username, n } => {
            last_n_videos(cx, username, n, tiktokapi::SubscriptionType::Likes).await
        }
        Command::LastVideo(username) => {
            last_n_videos(cx, username, 1, tiktokapi::SubscriptionType::CreatedVideos).await
        }
        Command::LastNVideo { username, n } => {
            last_n_videos(cx, username, n, tiktokapi::SubscriptionType::CreatedVideos).await
        }
        Command::SubscribeLikes(username) => {
            subscribe(username, &chat_id, tiktokapi::SubscriptionType::Likes).await
        }
        Command::SubscribeVideo(username) => {
            subscribe(
                username,
                &chat_id,
                tiktokapi::SubscriptionType::CreatedVideos,
            )
            .await
        }
        Command::UnsubscribeLikes(username) => {
            unsubscribe(username, &chat_id, tiktokapi::SubscriptionType::Likes).await
        }
        Command::UnsubscribeVideo(username) => {
            unsubscribe(
                username,
                &chat_id,
                tiktokapi::SubscriptionType::CreatedVideos,
            )
            .await
        }
    };
    log::info!("Command handling finished");
    status
}

async fn filter_sent_videos(
    db: &MongoDatabase,
    videos: Vec<tiktokapi::Video>,
    username: &str,
    stype: tiktokdb::SubscriptionType,
) -> Vec<tiktokapi::Video> {
    let to_remove: Vec<_> = join_all(videos.iter().map(|video| async {
        db.is_video_showed(&video.id, &username, stype)
            .await
            .unwrap_or(true)
    }))
    .await;

    videos
        .into_iter()
        .zip(to_remove.into_iter())
        .filter(|(_, f)| !*f)
        .map(|(v, _)| v)
        .collect::<Vec<_>>()
}

async fn get_videos_to_send(
    db: &MongoDatabase,
    username: &str,
    stype: tiktokdb::SubscriptionType,
) -> Result<Vec<tiktokapi::Video>, anyhow::Error> {
    let api = TiktokApi::from_env();
    let likes = api.get_content(&username, 5, stype).await?;
    log::info!("Received user {} likes", &username);
    Ok(filter_sent_videos(db, likes, &username, stype).await)
}

async fn send_video(
    bot: &AutoSend<Bot>,
    subscribed_info: &tiktokapi::UserInfo,
    chat_id: &str,
    video: &tiktokapi::Video,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error> {
    let chat_id: i64 = chat_id.parse().unwrap();
    let filename = format!("videos/{}.mp4", video.id);

    let message = bot
        .send_video(chat_id, InputFile::File(Path::new(&filename).to_path_buf()))
        .await?;
    if stype == SubscriptionType::Likes {
        bot.edit_message_caption(chat_id, message.id)
            .caption(format!(
                "User {} aka {} liked video from {} aka {}.\n\nDescription:\n{}",
                subscribed_info.unique_user_id,
                subscribed_info.nickname,
                video.unique_user_id,
                video.nickname,
                video.description
            ))
            .send()
            .await?;
    } else {
        bot.edit_message_caption(chat_id, message.id)
            .caption(format!(
                "User {} aka {} posted video.\n\nDescription:\n{}",
                video.unique_user_id, video.nickname, video.description
            ))
            .send()
            .await?;
    }
    Ok(())
}

async fn tiktok_updates_monitor_run(
    bot: &AutoSend<Bot>,
    db: &MongoDatabase,
) -> Result<(), anyhow::Error> {
    let users = db.get_users::<tiktokdb::User>().await?;
    let api = TiktokApi::from_env();

    for user in users {
        for stype in tiktokdb::SubscriptionType::iterator() {
            let chats = user.get_chats_by_subscription_type(stype);
            if chats.is_none() {
                continue;
            }
            let chats = chats.as_ref().unwrap();
            log::info!("User {} processing started.", &user.tiktok_username);
            let videos = get_videos_to_send(&db, &user.tiktok_username, stype).await?;
            download_videos(&videos).await;
            for video in videos {
                for chat in chats {
                    log::info!(
                        "Sending video from {} to chat {}",
                        user.tiktok_username,
                        chat
                    );
                    let tiktok_info = api.get_user_info(&user.tiktok_username).await?;
                    match send_video(&bot, &tiktok_info, chat, &video, stype).await {
                        Ok(_) => {
                            db.add_video(&video.id, &user.tiktok_username, stype)
                                .await?
                        }
                        Err(e) => log::error!(
                            "Error happened during sending video to {} with below error:\n{}",
                            &chat,
                            e
                        ),
                    }
                }
            }
            log::info!("User {} processing finished.", &user.tiktok_username);
        }
    }
    Ok(())
}

async fn tiktok_updates_monitoring_worker(bot: AutoSend<Bot>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
    loop {
        interval.tick().await;
        log::info!("Started updating TikTok feeds");
        tiktok_updates_monitor_run(&bot, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("Tiktok update run failed with an error: {}", e);
            });
        log::info!("Finished updating TikTok feeds");
    }
}

async fn bot_run() {
    let bot = Bot::from_env().auto_send();
    let bot_name: String = String::from("Tikitoki Likes");

    tokio::spawn(tiktok_updates_monitoring_worker(bot.clone()));
    teloxide::commands_repl(bot, bot_name, answer).await;
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    if let Err(e) = fs::create_dir_all("videos") {
        log::error!("Error: couldn't create videos directory.\n{}", e);
        return;
    }
    bot_run().await
}
