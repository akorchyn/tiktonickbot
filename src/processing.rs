pub(crate) mod bot;
pub(crate) mod updater;

use futures::future::join_all;
use futures::FutureExt;
use teloxide::prelude::*;
use teloxide::types::InputFile;

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use crate::api::tiktok as tiktokapi;
use crate::database::{tiktok as tiktokdb, MongoDatabase};

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("TIKTOK_BOT_MONGO_CON_STRING") {
        if let Ok(database) = env::var("TIKTOK_BOT_DATABASE_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

async fn send_video(
    bot: &AutoSend<Bot>,
    subscribed_info: &tiktokapi::UserInfo,
    chat_id: &str,
    video: &tiktokapi::Video,
    stype: tiktokapi::SubscriptionType,
) -> Result<(), anyhow::Error> {
    let chat_id: i64 = chat_id.parse().unwrap();
    let filename = format!("videos/{}.mp4", video.id);

    let message = bot
        .send_video(chat_id, InputFile::File(Path::new(&filename).to_path_buf()))
        .await?;
    if stype == tiktokapi::SubscriptionType::Likes {
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
