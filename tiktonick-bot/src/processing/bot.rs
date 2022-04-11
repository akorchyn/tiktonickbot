use std::sync::mpsc::SyncSender;
use teloxide::adaptors::Throttle;
use teloxide::{prelude2::*, utils::command::BotCommand};

use crate::api::instagram::InstagramAPI;
use crate::api::tiktok::TiktokAPI;
use crate::api::twitter::TwitterAPI;
use crate::api::*;
use crate::database::MongoDatabase;
use crate::processing::{LinkInfo, RequestModel, UserRequest};
use crate::regexp;

#[macro_use]
mod macros;
mod commands;

use commands::*;

generate_api_handler!(twitter_api_handler, TwitterAPI, TwitterCommands);
generate_api_handler!(instagram_api_handler, InstagramAPI, InstagramCommands);
generate_api_handler!(tiktok_api_handler, TiktokAPI, TiktokCommands);

#[derive(Clone)]
struct ConfigParameters {
    req_sender: SyncSender<UserRequest>,
}

pub(crate) async fn run(bot: AutoSend<Throttle<Bot>>, req_sender: SyncSender<UserRequest>) {
    let parameters = ConfigParameters { req_sender };

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<BasicCommands>()
                .endpoint(command_handling),
        )
        .branch(
            dptree::entry()
                .filter_command::<TwitterCommands>()
                .endpoint(twitter_api_handler),
        )
        .branch(
            dptree::entry()
                .filter_command::<InstagramCommands>()
                .endpoint(instagram_api_handler),
        )
        .branch(
            dptree::entry()
                .filter_command::<TiktokCommands>()
                .endpoint(tiktok_api_handler),
        )
        .branch(
            dptree::filter(|msg: Message| {
                let text = msg.text().unwrap_or_default();
                regexp::TWITTER_LINK.is_match(text)
                    || regexp::TIKTOK_FULL_LINK.is_match(text)
                    || regexp::TIKTOK_SHORT_LINK.is_match(text)
                    || regexp::INSTAGRAM_LINK.is_match(text)
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
    _: AutoSend<Throttle<Bot>>,
    cfg: ConfigParameters,
) -> anyhow::Result<()> {
    let chat_id = message.chat.id.to_string();
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
        (regexp::INSTAGRAM_LINK.find_iter(text), Api::Instagram),
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
    bot: AutoSend<Throttle<Bot>>,
    command: BasicCommands,
) -> anyhow::Result<()> {
    let chat_id = message.chat.id.to_string();
    let status = match command {
        BasicCommands::Help => {
            let text = format!(
                "{}\n{}\n{}\n{}",
                BasicCommands::descriptions(),
                TwitterCommands::descriptions(),
                InstagramCommands::descriptions(),
                TiktokCommands::descriptions()
            );
            bot.send_message(message.chat.id, text).await?;
            Ok(())
        }
        BasicCommands::Subscriptions => show_subscriptions(&bot, &chat_id).await,
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

async fn show_subscriptions(bot: &AutoSend<Throttle<Bot>>, chat_id: &str) -> anyhow::Result<()> {
    let db = super::create_db().await?;
    let sub_string = vec![
        get_subscription_string_for_api::<TwitterAPI>(chat_id, &db).await?,
        get_subscription_string_for_api::<InstagramAPI>(chat_id, &db).await?,
        get_subscription_string_for_api::<TiktokAPI>(chat_id, &db).await?,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<String>>()
    .join("");

    if !sub_string.is_empty() {
        bot.send_message(
            chat_id.parse::<i64>().unwrap(),
            format!("Group subscriptions:\n {}", sub_string),
        )
        .await?;
    } else {
        let empty_text = "Currently, group doesn't have any subscriptions";
        bot.send_message(chat_id.parse::<i64>().unwrap(), empty_text)
            .await?;
    }
    Ok(())
}

async fn last_n_data<Api>(
    bot: &AutoSend<Throttle<Bot>>,
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
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload,
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
    bot: &AutoSend<Throttle<Bot>>,
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
    cx: &AutoSend<Throttle<Bot>>,
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
