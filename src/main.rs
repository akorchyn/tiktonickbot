use futures::future::join_all;
use futures::FutureExt;
use reqwest;
use reqwest::header::{HeaderMap, REFERER, USER_AGENT};
use serde::{self, Deserialize};
use serde_json;

use reqwest::Response;
use std::fs::{self, File};
use std::io;
use std::path::Path;

fn default_verify_fp() -> &'static str {
    "verify_38576c173a44b96c30ce3f5a6092480a"
}

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.93 Safari/537.36".parse().unwrap());
    headers.insert(
        "cookie",
        format!("s_v_web_id={}", default_verify_fp())
            .parse()
            .unwrap(),
    );
    headers.insert(
        REFERER,
        "referer: 'https://www.tiktok.com/'".parse().unwrap(),
    );
    return headers;
}

struct LikedVideo {
    id: String,
    unique_user_id: String,
    nickname: String,
    download_address: String,
    description: String,
}

#[derive(Deserialize, Debug)]
struct Video {
    id: String,
    #[serde(rename = "downloadAddr")]
    download_address: String,
}

#[derive(Deserialize, Debug)]
struct Author {
    #[serde(rename = "uniqueId")]
    unique_user_id: String,
    #[serde(rename = "nickname")]
    nickname: String,
}

#[derive(Deserialize, Debug)]
struct LikeItem {
    video: Video,
    author: Author,
    #[serde(rename = "desc")]
    description: String,
}

#[derive(Deserialize, Debug)]
struct LikesResponse {
    #[serde(rename = "itemList")]
    item_list: Option<Vec<LikeItem>>,
}

#[derive(Deserialize, Debug)]
struct UserInfo {
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
    user_info: &UserInfo,
    cursor: u32,
    count: u32,
) -> Result<Vec<LikedVideo>, String> {
    let body = send_request_with_default_headers(&format!("https://m.tiktok.com/api/favorite/item_list/?aid={}&verifyFp={}&cursor={}&count={}&secUid={}", 1988, default_verify_fp(), cursor, count, user_info.sec_uid)).await;
    match body {
        Ok(response) => {
            let text = response.text().await.unwrap_or("".to_string());
            match serde_json::from_str::<LikesResponse>(&text) {
                Ok(likes) => Ok(likes
                    .item_list
                    .unwrap_or(Vec::new())
                    .into_iter()
                    .map(|item| LikedVideo {
                        id: item.video.id,
                        unique_user_id: item.author.unique_user_id,
                        nickname: item.author.nickname,
                        description: item.description,
                        download_address: item.video.download_address,
                    })
                    .collect()),
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

async fn receive_user_info_by_login(login: &str) -> Result<UserInfo, String> {
    let body =
        send_request_with_default_headers(&format!("https://www.tiktok.com/@{}?", login)).await;

    match body {
        Ok(response) => {
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
                match serde_json::from_str::<UserInfo>(json_text) {
                    Ok(user_info) => Ok(user_info),
                    Err(e) => Err(format!("{}", e.to_string())),
                }
            } else {
                Err(format!(
                    "Internal Error: Don't find. Json start -> {} Json end -> {}",
                    json_start, json_end
                ))
            }
        }
        Err(_) => Err("Internal Error: Request retrieval problem".to_string()),
    }
}

async fn download_videos(liked_videos: &Vec<LikedVideo>) {
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

use teloxide::types::InputFile;
use teloxide::{prelude::*, utils::command::BotCommand};

async fn last_n_videos(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    username: String,
    n: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let user_info = receive_user_info_by_login(&username).await?;
    let liked_videos = receive_user_likes(&user_info, 0, n.into()).await?;
    download_videos(&liked_videos).await;
    for video in liked_videos {
        let filename = format!("videos/{}.mp4", video.id);

        if let Err(_) = cx
            .answer(format!(
                "User {} aka {} liked video from {} aka {}.\n\nDescription:\n{}",
                user_info.unique_user_id,
                user_info.nickname,
                video.unique_user_id,
                video.nickname,
                video.description
            ))
            .await
        {
            log::error!("Error: Failed to send a video");
        }

        if let Err(_) = cx
            .answer_video(InputFile::File(Path::new(&filename).to_path_buf()))
            .await
        {
            log::error!("Error: Failed to send a video");
        }
    }
    Ok(())
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
}

async fn answer(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    command: Command,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        Command::Help => {
            cx.answer(Command::descriptions()).await?;
            Ok(())
        }
        Command::LastLike(username) => last_n_videos(cx, username, 1).await,
        Command::LastNLike { username, n } => last_n_videos(cx, username, n).await,
    }
}

async fn run() {
    let bot = Bot::from_env().auto_send();
    let bot_name: String = String::from("Tikitoki Likes");
    teloxide::commands_repl(bot, bot_name, answer).await;
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    if let Err(e) = fs::create_dir_all("videos") {
        log::error!("Error: couldn't create videos directory.\n{}", e);
        return;
    }
    run().await
}
