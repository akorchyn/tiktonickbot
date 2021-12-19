pub(crate) mod bot;
pub(crate) mod updater;

use futures::future::join_all;
use futures::FutureExt;
use teloxide::prelude::*;
use teloxide::types::{InputFile, InputMedia, InputMediaPhoto, InputMediaVideo};

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use crate::api::{
    tiktok as tiktokapi, DataForDownload, GenerateSubscriptionMessage, ReturnTextInfo,
    ReturnUserInfo,
};
use crate::api::{twitter as twitterapi, ReturnDataForDownload};
use crate::database::{tiktok as tiktokdb, MongoDatabase};

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("TIKTOK_BOT_MONGO_CON_STRING") {
        if let Ok(database) = env::var("TIKTOK_BOT_DATABASE_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

async fn send_content<UserInfo, Content, SubscriptionType>(
    bot: &AutoSend<Bot>,
    user_info: &UserInfo,
    chat_id: &str,
    content: &Content,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    UserInfo: ReturnUserInfo,
    Content: ReturnDataForDownload + ReturnTextInfo,
    SubscriptionType: GenerateSubscriptionMessage<UserInfo, Content>,
{
    let chat_id: i64 = chat_id.parse().unwrap();
    if !content.is_data_for_download() {
        bot.send_message(chat_id, stype.subscription_message(&user_info, &content))
            .await?;
    } else {
        let media: Vec<InputMedia> = content
            .data()
            .into_iter()
            .map(|item| {
                let filename = format!("content/{}.{}", item.name, item.data_type.to_extension());
                let input_file = InputFile::File(Path::new(&filename).to_path_buf());
                match item.data_type {
                    crate::api::DataType::Image => InputMedia::Photo(
                        InputMediaPhoto::new(input_file)
                            .caption(stype.subscription_message(&user_info, &content)),
                    ),
                    crate::api::DataType::Video => InputMedia::Video(
                        InputMediaVideo::new(input_file)
                            .caption(stype.subscription_message(&user_info, &content)),
                    ),
                }
            })
            .collect();
        bot.send_media_group(chat_id, media).await?;
    }
    Ok(())
}

async fn download(content: &DataForDownload) -> Result<(), anyhow::Error> {
    let filename = format!(
        "content/{}.{}",
        content.name,
        content.data_type.to_extension()
    );
    if Path::new(&filename).exists() {
        log::info!("Content is already cached({}). Skipping...", filename);
        return Ok(());
    }
    log::info!("Downloading content");
    let status = reqwest::get(&content.url)
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
        .await;
    if !status {
        Err(anyhow::anyhow!("Failed to download"))
    } else {
        Ok(())
    }
}

async fn download_content<T>(content: &Vec<T>)
where
    T: crate::api::ReturnDataForDownload,
{
    let futures: Vec<_> = content
        .into_iter()
        .map(|content| async {
            let content = content.data();
            let futures: Vec<_> = content
                .iter()
                .map(|content| async {
                    download(content)
                        .await
                        .unwrap_or_else(|e| log::error!("Failed to download: {}", e.to_string()));
                })
                .collect();
            join_all(futures).await;
        })
        .collect();
    join_all(futures).await;
}
