pub(crate) mod bot;
pub(crate) mod updater;

use futures::future::join_all;
use futures::FutureExt;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, InputFile, InputMedia, InputMediaPhoto, InputMediaVideo, ParseMode, UserId,
};

use std::env;
use std::fs::File;
use std::io;
use std::mem::take;
use std::path::Path;
use teloxide::adaptors::{AutoSend, Throttle};

use crate::api::{
    Api, DataForDownload, OutputType, PrepareDescription, ReturnDataForDownload, ReturnUserInfo,
    SubscriptionType,
};
use crate::common::description_builder::DescriptionBuilder;
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
    bot: &AutoSend<Throttle<Bot>>,
    user_info: &UserInfo,
    chat_id: &str,
    content: &Content,
    output_type: OutputType,
) -> Result<(), anyhow::Error>
where
    Api: PrepareDescription<UserInfo, Content>,
    UserInfo: ReturnUserInfo,
    Content: ReturnDataForDownload,
{
    let chat_id = ChatId(chat_id.parse().unwrap());
    let mut builder = Api::prepare_description(user_info, content, &output_type);
    let send_text_message = |builder: DescriptionBuilder| {
        let text = builder.build();
        bot.send_message(chat_id, text)
            .parse_mode(ParseMode::Html)
            .disable_web_page_preview(true)
            .disable_notification(true)
    };

    if !content.is_data_for_download() {
        builder.size_limit(4096);
        send_text_message(builder).await?;
    } else {
        let data = content.data();
        let file_sizes = data.iter().fold(0, |acc, item| {
            let filename = format!("content/{}.{}", item.name, item.data_type.to_extension());
            acc + Path::new(&filename)
                .metadata()
                .map_or(100 * 1024 * 1024, |m| m.len())
        });
        // Telegram has file size limitation for bots.
        if file_sizes > 50 * 1024 * 1024 {
            builder.size_limit(4096).achieved_content_size_limit(true);
            send_text_message(builder).await?;
        } else {
            let mut caption = Some(builder.size_limit(1024).build());
            let media: Vec<InputMedia> = content
                .data()
                .into_iter()
                .map(|item| {
                    let filename =
                        format!("content/{}.{}", item.name, item.data_type.to_extension());
                    let media = InputFile::file(Path::new(&filename));
                    match item.data_type {
                        crate::api::DataType::Image => InputMedia::Photo(InputMediaPhoto {
                            media,
                            caption: take(&mut caption),
                            parse_mode: Some(ParseMode::Html),
                            caption_entities: None,
                        }),
                        crate::api::DataType::Video => InputMedia::Video(InputMediaVideo {
                            media,
                            thumb: None,
                            caption: take(&mut caption),
                            parse_mode: Some(ParseMode::Html),
                            caption_entities: None,
                            width: None,
                            height: None,
                            duration: None,
                            supports_streaming: None,
                        }),
                    }
                })
                .collect();
            bot.send_media_group(chat_id, media)
                .disable_notification(true)
                .await?;
        }
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
                        return io::copy(&mut bytes.as_ref(), &mut file).is_ok();
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

async fn download_content<T>(content: ContentForDownload<'_, T>)
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
                .iter()
                .map(|content| async { iterate_through_data_for_download(content.data()).await })
                .collect();
            join_all(futures).await;
        }
        ContentForDownload::Element(elem) => {
            iterate_through_data_for_download(elem.data()).await;
        }
    }
}
