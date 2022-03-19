pub(crate) mod bot;
pub(crate) mod updater;

use futures::future::join_all;
use futures::FutureExt;
use teloxide::prelude2::*;
use teloxide::types::{InputFile, InputMedia, InputMediaPhoto, InputMediaVideo};

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use crate::api::{
    Api, DataForDownload, GenerateMessage, OutputType, ReturnDataForDownload, ReturnTextInfo,
    ReturnUserInfo, SubscriptionType,
};
use crate::database::MongoDatabase;

#[derive(Debug)]
pub(crate) struct RequestModel {
    chat_id: String,
    stype: SubscriptionType,
    user: String,
    api: Api,
}

#[derive(Debug)]
pub(crate) struct LinkInfo {
    chat_id: String,
    telegram_user: crate::api::TelegramUser,
    api: Api,
    link: String,
}

pub(crate) enum UserRequest {
    LastNData(RequestModel, u8),
    Subscribe(RequestModel),
    ProcessLink(LinkInfo),
}

async fn create_db() -> Result<MongoDatabase, anyhow::Error> {
    if let Ok(con) = env::var("TIKTOK_BOT_MONGO_CON_STRING") {
        if let Ok(database) = env::var("TIKTOK_BOT_DATABASE_NAME") {
            return MongoDatabase::from_connection_string(&con, &database).await;
        }
    }
    panic!("TIKTOK_BOT_MONGO_CON_STRING & TIKTOK_BOT_DATABASE_NAME env variables don't exist");
}

async fn send_content<Api, UserInfo, Content>(
    _: &Api,
    bot: &AutoSend<Bot>,
    user_info: &UserInfo,
    chat_id: &str,
    content: &Content,
    output_type: OutputType,
) -> Result<(), anyhow::Error>
where
    Api: GenerateMessage<UserInfo, Content>,
    UserInfo: ReturnUserInfo,
    Content: ReturnDataForDownload + ReturnTextInfo,
{
    let chat_id: i64 = chat_id.parse().unwrap();
    if !content.is_data_for_download() {
        (if let Some(v) = Api::message_format() {
            bot.send_message(chat_id, Api::message(&user_info, &content, &output_type))
                .parse_mode(v)
        } else {
            bot.send_message(chat_id, Api::message(&user_info, &content, &output_type))
        })
        .disable_web_page_preview(true)
        .await?;
    } else {
        let mut is_first = true;
        let media: Vec<InputMedia> = content
            .data()
            .into_iter()
            .map(|item| {
                let filename = format!("content/{}.{}", item.name, item.data_type.to_extension());
                let media = InputFile::file(Path::new(&filename));
                let caption = if is_first {
                    is_first = false;
                    Some(Api::message(&user_info, &content, &output_type))
                } else {
                    None
                };
                match item.data_type {
                    crate::api::DataType::Image => InputMedia::Photo(InputMediaPhoto {
                        media,
                        caption,
                        parse_mode: Api::message_format(),
                        caption_entities: None,
                    }),
                    crate::api::DataType::Video => InputMedia::Video(InputMediaVideo {
                        media,
                        thumb: None,
                        caption,
                        parse_mode: Api::message_format(),
                        caption_entities: None,
                        width: None,
                        height: None,
                        duration: None,
                        supports_streaming: None,
                    }),
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

enum ContentForDownload<'a, T> {
    Array(&'a Vec<T>),
    Element(&'a T),
}

async fn download_content<'a, T>(content: ContentForDownload<'a, T>)
where
    T: crate::api::ReturnDataForDownload,
{
    async fn iterate_through_data_for_download(content: Vec<DataForDownload>) {
        let futures: Vec<_> = content
            .iter()
            .map(|content| async {
                download(content)
                    .await
                    .unwrap_or_else(|e| log::error!("Failed to download: {}", e.to_string()));
            })
            .collect();
        join_all(futures).await;
    }

    match content {
        ContentForDownload::Array(content) => {
            let futures: Vec<_> = content
                .into_iter()
                .map(|content| async { iterate_through_data_for_download(content.data()).await })
                .collect();
            join_all(futures).await;
        }
        ContentForDownload::Element(elem) => {
            iterate_through_data_for_download(elem.data()).await;
        }
    }
}
