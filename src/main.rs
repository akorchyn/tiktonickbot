use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;

use futures::future::join_all;
use futures::FutureExt;
use reqwest;
use reqwest::header::{HeaderMap, REFERER, USER_AGENT};
use reqwest::Response;
use serde::{self, Deserialize};
use serde_json;
use teloxide::types::InputFile;
use teloxide::{prelude::*, utils::command::BotCommand};

use anyhow;
use database::MongoDatabase;

use crate::database::{SubscriptionType, TiktokDatabaseApi};

mod database;

fn default_verify_fp() -> String {
    env::var("VERIFY_FP").unwrap()
}

fn default_cookie() -> String {
    env::var("COOKIE").unwrap()
}

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.93 Safari/537.36".parse().unwrap());
    headers.insert("cookie", default_cookie().parse().unwrap());
    headers.insert(
        REFERER,
        "referer: 'https://www.tiktok.com/'".parse().unwrap(),
    );
    return headers;
}

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("TIKTOK_BOT_MONGO_CON_STRING") {
        if let Ok(database) = env::var("TIKTOK_BOT_DATABASE_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

#[derive(Debug)]
struct Video {
    id: String,
    unique_user_id: String,
    nickname: String,
    download_address: String,
    description: String,
}

#[derive(Deserialize, Debug)]
struct TiktokVideo {
    id: String,
    #[serde(rename = "downloadAddr")]
    download_address: String,
}

#[derive(Deserialize, Debug)]
struct TiktokAuthor {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
}

#[derive(Deserialize, Debug)]
struct TiktokItem {
    video: TiktokVideo,
    author: TiktokAuthor,
    #[serde(rename = "desc")]
    description: String,
}

#[derive(Deserialize, Debug)]
struct TiktokFeedResponse {
    #[serde(rename = "itemList")]
    item_list: Option<Vec<TiktokItem>>,
}

#[derive(Deserialize, Debug)]
struct TiktokUserInfo {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
    #[serde(rename = "secUid")]
    sec_uid: String,
}

async fn send_request_with_default_headers(url: &str) -> Result<Response, reqwest::Error> {
    let client = reqwest::Client::new();
    client.get(url).headers(default_headers()).send().await
}

async fn receive_user_likes(
    user_info: &TiktokUserInfo,
    cursor: u32,
    count: u32,
) -> Result<Vec<Video>, anyhow::Error> {
    let response = send_request_with_default_headers(&format!("https://m.tiktok.com/api/favorite/item_list/?aid={}&verifyFp={}&cursor={}&count={}&secUid={}", 1988, default_verify_fp(), cursor, count, user_info.sec_uid)).await?;
    let text = response.text().await.unwrap_or("".to_string());
    let likes = serde_json::from_str::<TiktokFeedResponse>(&text)?;
    Ok(likes
        .item_list
        .unwrap_or(Vec::new())
        .into_iter()
        .map(|item| Video {
            id: item.video.id,
            unique_user_id: item.author.unique_user_id,
            nickname: item.author.nickname,
            description: item.description,
            download_address: item.video.download_address,
        })
        .collect())
}

async fn receive_user_info_by_login(username: &str) -> Result<TiktokUserInfo, anyhow::Error> {
    let response =
        send_request_with_default_headers(&format!("https://www.tiktok.com/@{}?", username))
            .await?;
    let start_pattern = "{\"user\":";
    let end_pattern = "},\"stats\":";
    let text = response.text().await.unwrap_or("".to_string());
    let json_start = text
        .find(start_pattern)
        .and_then(|x| Some(x + start_pattern.len()))
        .unwrap_or(0);
    let json_end = text[json_start..]
        .find(end_pattern)
        .and_then(|x| Some(x + json_start + 1))
        .unwrap_or(0);

    if json_start != 0 && json_start < json_end {
        let json_text = &text[json_start..json_end];
        let user_info = serde_json::from_str::<TiktokUserInfo>(json_text)?;
        Ok(user_info)
    } else {
        Err(anyhow::anyhow!(
            "Internal Error: Failed with user {}. Json start -> {} Json end -> {}\nResponse: {}",
            username,
            json_start,
            json_end,
            text
        ))
    }
}

async fn download_videos(liked_videos: &Vec<Video>) {
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
            send_request_with_default_headers(&video.download_address)
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
) -> Result<(), anyhow::Error> {
    let user_info = receive_user_info_by_login(&username).await?;
    let liked_videos = receive_user_likes(&user_info, 0, n.into()).await?;
    let mut succeed = true;
    download_videos(&liked_videos).await;
    for video in liked_videos {
        send_video(&cx.requester, &user_info, &cx.update.chat.id.to_string(), &video)
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
    #[command(description = "subscribe chat to tiktok user likes feed.")]
    SubscribeLikes(String),
    #[command(description = "subscribe chat to tiktok user likes feed.")]
    SubscribeContent(String),
    #[command(description = "subscribe chat from tiktok user likes feed.")]
    UnsubscribeLikes(String),
    #[command(description = "subscribe chat from tiktok user likes feed.")]
    UnsubscribeContent(String),
}

async fn subscribe(
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error> {
    let db = create_db().await?;
    let user_info = receive_user_info_by_login(&username).await?;
    let likes = receive_user_likes(&user_info, 0, 5).await?;
    for like in likes {
        db.add_video(&like.id, &user_info.unique_user_id, stype)
            .await?;
    }
    db.subscribe_user(&username, &chat_id, stype).await
}

async fn unsubscribe(
    username: String,
    chat_id: &str,
    stype: SubscriptionType,
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
        Command::LastLike(username) => last_n_videos(cx, username, 1).await,
        Command::LastNLike { username, n } => last_n_videos(cx, username, n).await,
        Command::SubscribeLikes(username) => {
            subscribe(username, &chat_id, SubscriptionType::TiktokLikes).await
        }
        Command::SubscribeContent(username) => {
            Ok(())
            // subscribe(username, &chat_id, SubscriptionType::TiktokContent).await
        }
        Command::UnsubscribeLikes(username) => {
            unsubscribe(username, &chat_id, SubscriptionType::TiktokLikes).await
        }
        Command::UnsubscribeContent(username) => {
            Ok(())
            // unsubscribe(username, &chat_id, SubscriptionType::TiktokContent).await
        }
    };
    log::info!("Command handling finished");
    status
}

async fn filter_sent_videos(
    db: &MongoDatabase,
    videos: Vec<Video>,
    username: &str,
    stype: SubscriptionType,
) -> Vec<Video> {
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
    user: &TiktokUserInfo,
    stype: SubscriptionType,
) -> Result<Vec<Video>, anyhow::Error> {
    let likes = receive_user_likes(&user, 0, 5).await?;
    log::info!("Received user {} likes", &user.unique_user_id);
    Ok(filter_sent_videos(db, likes, &user.unique_user_id, stype).await)
}

async fn send_video(
    bot: &AutoSend<Bot>,
    subscribed_info: &TiktokUserInfo,
    chat_id: &str,
    video: &Video,
) -> Result<(), anyhow::Error> {
    let chat_id: i64 = chat_id.parse().unwrap();
    let filename = format!("videos/{}.mp4", video.id);

    let message = bot
        .send_video(chat_id, InputFile::File(Path::new(&filename).to_path_buf()))
        .await?;
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
    Ok(())
}

async fn tiktok_updates_monitor_run(
    bot: &AutoSend<Bot>,
    db: &MongoDatabase,
) -> Result<(), anyhow::Error> {
    let users = db.get_users().await?;

    for user in users {
        for stype in SubscriptionType::iterator() {
            let chats = user.get_chats_by_subscription_type(stype);
            if chats.is_none() {
                continue;
            }
            let chats = chats.as_ref().unwrap();
            log::info!("User {} processing started.", &user.tiktok_username);
            let tiktok_info = receive_user_info_by_login(&user.tiktok_username).await?;
            log::info!("Received user {} meta-info", user.tiktok_username);
            let videos = get_videos_to_send(&db, &tiktok_info, stype).await?;
            download_videos(&videos).await;
            for video in videos {
                for chat in chats {
                    log::info!(
                        "Sending video from {} to chat {}",
                        user.tiktok_username,
                        chat
                    );
                    match send_video(&bot, &tiktok_info, chat, &video).await {
                        Ok(_) => db.add_video(&video.id, &user.tiktok_username, stype).await?,
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

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
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
