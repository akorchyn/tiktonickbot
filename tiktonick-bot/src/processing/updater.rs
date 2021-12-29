use crate::api::*;

use teloxide::prelude::*;

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use crate::database::User;
use futures::future::join_all;
use std::sync::mpsc;
use teloxide::types::ChatId;

use super::*;

pub(crate) async fn run(bot: AutoSend<Bot>, request_queue: mpsc::Receiver<UserRequest>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
    let tiktok_api = TiktokApi::from_env();
    let twitter_api = TwitterApi::from_env();
    let mut requests: Vec<UserRequest> = Vec::new();

    loop {
        interval.tick().await;
        log::info!("Processing queue");
        fill_queue(&mut requests, &request_queue);
        process_queue(&bot, &db, &mut requests).await;
        log::info!("Started updating Twitter feeds");
        updates_monitor_run(&bot, &twitter_api, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("Twitter update run failed with an error: {}", e);
            });
        updates_monitor_run(&bot, &tiktok_api, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("Twitter update run failed with an error: {}", e);
            });
    }
}

fn fill_queue(sub_queue: &mut Vec<UserRequest>, sub_receiver: &mpsc::Receiver<UserRequest>) {
    log::info!("Starting retrieval");
    for i in sub_receiver.try_iter() {
        sub_queue.push(i);
    }
    log::info!("Finished retrieval");
}

async fn process_queue(bot: &AutoSend<Bot>, db: &MongoDatabase, req_queue: &mut Vec<UserRequest>) {
    log::info!("Queue size is: {}", req_queue.len());
    let req_queue_processed = req_queue
        .drain(..)
        .map(|request| async {
            let status = match &request {
                UserRequest::LastNData(r, n) => match r.api {
                    Api::Twitter => last_n_data::<TwitterApi>(&bot, r, *n).await,
                    Api::Tiktok => last_n_data::<TiktokApi>(&bot, r, *n).await,
                },
                UserRequest::Subscribe(r) => match r.api {
                    Api::Twitter => subscribe::<TwitterApi>(&bot, &db, r).await,
                    Api::Tiktok => subscribe::<TiktokApi>(&bot, &db, r).await,
                },
            };
            if let Err(e) = &status {
                log::error!("Failed to process command: {}", e.to_string());
            }
            (request, status.is_err())
        })
        .collect::<Vec<_>>();
    let req_queue_processed: Vec<(UserRequest, bool)> = join_all(req_queue_processed).await;
    *req_queue = req_queue_processed
        .into_iter()
        .filter(|(_, to_keep)| *to_keep)
        .map(|(subscription, _)| subscription)
        .collect::<Vec<UserRequest>>();
    log::info!("Queue size after is: {}", req_queue.len());
}

async fn last_n_data<Api>(
    bot: &AutoSend<Bot>,
    request_model: &RequestModel,
    n: u8,
) -> Result<(), anyhow::Error>
where
    Api: ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    let api = Api::from_env();
    let user_info = api.get_user_info(&request_model.user).await?;
    if let Some(user_info) = user_info {
        let mut data = api
            .get_content(&user_info.id(), n, request_model.stype)
            .await?;
        data.truncate(n as usize);
        super::download_content(&data).await;
        for i in data.into_iter().rev() {
            super::send_content(
                &api,
                &bot,
                &user_info,
                &request_model.chat_id,
                &i,
                request_model.stype,
            )
            .await?;
        }
    } else {
        let chat_id: i64 = request_model.chat_id.parse().unwrap();
        bot.send_message(chat_id, format!("@{} user not found.", &request_model.user))
            .await?;
    }
    Ok(())
}

async fn subscribe<Api>(
    bot: &AutoSend<Bot>,
    db: &MongoDatabase,
    model: &RequestModel,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + ApiAlive
        + Sync
        + Send,
    <Api as ApiContentReceiver>::Out: GetId,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    if db
        .is_user_subscribed::<Api>(&model.user, &model.chat_id, model.stype)
        .await?
    {
        Ok(())
    } else {
        let chat_id = model.chat_id.parse().unwrap_or(0i64);
        let api = Api::from_env();
        let user_info = api.get_user_info(&model.user).await?;
        if let Some(user_info) = user_info {
            if !db.is_user_exist::<Api>(&model.user, model.stype).await? {
                let content = api.get_content(&user_info.id(), 5, model.stype).await?;
                for item in content {
                    db.add_content::<Api>(&item.id(), &model.user, model.stype)
                        .await?;
                }
            }
            db.subscribe_user::<Api>(&user_info.username(), &model.chat_id, model.stype)
                .await?;
            bot.send_message(
                ChatId::Id(chat_id),
                format!(
                    "Successfully subscribed to {} aka {}",
                    &model.user,
                    &user_info.nickname()
                ),
            )
            .await?;
        } else {
            bot.send_message(
                ChatId::Id(chat_id),
                format!(
                    "Failed to subscribe for user: @{}. User not found",
                    &model.user
                ),
            )
            .await?;
        }
        Ok(())
    }
}

async fn process_user<Api>(
    bot: &AutoSend<Bot>,
    api: &Api,
    db: &MongoDatabase,
    user: &User,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    <Api as ApiContentReceiver>::Out: GetId + ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    Api: DatabaseInfoProvider
        + ApiAlive
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
{
    if !api.is_alive().await {
        log::info!(
            "{} Api is dead, trying make it alive. Will try update on next iteration",
            Api::name()
        );
        api.try_make_alive().await?;
        return Ok(());
    }

    let chats = user.get_chats_by_subscription_type(stype);
    if chats.is_none() {
        return Ok(());
    }
    let chats = chats.as_ref().unwrap();
    if chats.is_empty() {
        return Ok(());
    }
    let user_info = api.get_user_info(&user.id).await?;
    if let Some(user_info) = user_info {
        log::info!("{}: User {} processing started.", Api::name(), &user.id);
        let content = api.get_content(&user_info.id(), 5, stype).await?;
        log::info!("{}: Received user {} data", Api::name(), &user.id);
        let content = filter_sent_videos::<Api, <Api as ApiContentReceiver>::Out>(
            db, content, &user.id, stype,
        )
        .await;
        download_content(&content).await;
        for element in content.into_iter().rev() {
            let mut succeed = false;
            for chat in chats {
                log::info!(
                    "{}: Sending video from {} to chat {}",
                    Api::name(),
                    user.id,
                    chat
                );
                match send_content::<
                    Api,
                    <Api as ApiUserInfoReceiver>::Out,
                    <Api as ApiContentReceiver>::Out,
                >(&api, &bot, &user_info, chat, &element, stype)
                .await
                {
                    Ok(_) => succeed = true,
                    Err(e) => log::error!(
                        "{}: Error happened during sending video to {} with below error:\n{}",
                        Api::name(),
                        &chat,
                        e
                    ),
                }
            }
            if succeed {
                db.add_content::<Api>(element.id(), &user.id, stype).await?
            }
        }
    }
    Ok(())
}

async fn updates_monitor_run<Api>(
    bot: &AutoSend<Bot>,
    api: &Api,
    db: &MongoDatabase,
) -> Result<(), anyhow::Error>
where
    <Api as ApiContentReceiver>::Out: GetId + ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    Api: DatabaseInfoProvider
        + ApiAlive
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
{
    let users = db.get_collection::<Api, crate::database::User>().await?;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
    for user in users {
        for stype in SubscriptionType::iterator() {
            interval.tick().await;
            process_user::<Api>(&bot, &api, &db, &user, stype)
                .await
                .unwrap_or_else(|e| {
                    log::warn!("{}: Failed to process user: {}", Api::name(), e.to_string());
                });
            log::info!("{}: User {} processing finished.", Api::name(), &user.id);
        }
    }
    Ok(())
}

async fn filter_sent_videos<Api: DatabaseInfoProvider, T: GetId>(
    db: &MongoDatabase,
    content: Vec<T>,
    user_id: &str,
    stype: SubscriptionType,
) -> Vec<T> {
    let to_remove: Vec<_> = join_all(content.iter().map(|elem| async {
        db.is_content_showed::<Api>(&elem.id(), &user_id, stype)
            .await
            .unwrap_or(true)
    }))
    .await;

    content
        .into_iter()
        .zip(to_remove.into_iter())
        .filter(|(_, f)| !*f)
        .map(|(v, _)| v)
        .collect::<Vec<_>>()
}
