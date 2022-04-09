use crate::api::*;

use teloxide::prelude2::*;

use crate::api::instagram::InstagramAPI;
use crate::api::tiktok::TiktokAPI;
use crate::api::twitter::TwitterAPI;
use crate::database::User;
use futures::future::join_all;
use std::sync::mpsc;
use teloxide::adaptors::Throttle;
use teloxide::types::ChatId;

use super::*;

pub(crate) async fn run(bot: AutoSend<Throttle<Bot>>, request_queue: mpsc::Receiver<UserRequest>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");
    let tiktok_api = TiktokAPI::from_env();
    let twitter_api = TwitterAPI::from_env();
    let instagram_api = InstagramAPI::from_env();
    tokio::spawn(update_loop_handler(bot.clone(), tiktok_api, db.clone()));
    tokio::spawn(update_loop_handler(bot.clone(), twitter_api, db.clone()));
    tokio::spawn(update_loop_handler(bot.clone(), instagram_api, db.clone()));
    request_handler(bot, request_queue, db).await;
}

async fn request_handler(
    bot: AutoSend<Throttle<Bot>>,
    request_queue: mpsc::Receiver<UserRequest>,
    db: MongoDatabase,
) {
    let mut requests: Vec<UserRequest> = Vec::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
    loop {
        interval.tick().await;
        log::info!("Started processing queue");
        fill_queue(&mut requests, &request_queue);
        process_queue(&bot, &db, &mut requests).await;
    }
}

async fn update_loop_handler<Api>(bot: AutoSend<Throttle<Bot>>, api: Api, db: MongoDatabase)
where
    <Api as ApiContentReceiver>::Out: GetId + ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    Api: DatabaseInfoProvider
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
{
    // Todo: receive delay from api type.
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        updates_monitor_run(&bot, &api, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("{} update run failed with an error: {}", Api::name(), e);
            });
    }
}

fn fill_queue(sub_queue: &mut Vec<UserRequest>, sub_receiver: &mpsc::Receiver<UserRequest>) {
    log::info!("Starting requests retrieval");
    for i in sub_receiver.try_iter() {
        sub_queue.push(i);
    }
    log::info!("Finished requests retrieval");
}

async fn process_queue(
    bot: &AutoSend<Throttle<Bot>>,
    db: &MongoDatabase,
    req_queue: &mut Vec<UserRequest>,
) {
    log::info!("Queue size is: {}", req_queue.len());
    let req_queue_processed = req_queue
        .drain(..)
        .map(|request| async {
            let status = match &request {
                UserRequest::LastNData(r, n) => match r.api {
                    Api::Twitter => last_n_data::<TwitterAPI>(bot, r, *n).await,
                    Api::Tiktok => last_n_data::<TiktokAPI>(bot, r, *n).await,
                    Api::Instagram => last_n_data::<InstagramAPI>(bot, r, *n).await,
                },
                UserRequest::Subscribe(r) => match r.api {
                    Api::Twitter => subscribe::<TwitterAPI>(bot, db, r).await,
                    Api::Tiktok => subscribe::<TiktokAPI>(bot, db, r).await,
                    Api::Instagram => subscribe::<InstagramAPI>(bot, db, r).await,
                },
                UserRequest::ProcessLink(l) => match l.api {
                    Api::Twitter => process_link::<TwitterAPI>(bot, l).await,
                    Api::Tiktok => process_link::<TiktokAPI>(bot, l).await,
                    Api::Instagram => process_link::<InstagramAPI>(bot, l).await,
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

async fn process_link<Api>(
    bot: &AutoSend<Throttle<Bot>>,
    link_info: &LinkInfo,
) -> anyhow::Result<()>
where
    Api: ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo + ReturnUsername,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    let api = Api::from_env();
    let object = api.get_content_for_link(&link_info.link).await?;
    log::info!("Fetching {} user data", object.username());
    let user_info = api.get_user_info(object.username()).await?;
    if let Some(user_info) = user_info {
        super::download_content(ContentForDownload::Element(&object)).await;
        super::send_content(
            &api,
            bot,
            &user_info,
            &link_info.chat_id,
            &object,
            OutputType::ByLink(link_info.telegram_user.clone()),
        )
        .await?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to fetch user data"))
    }
}

async fn last_n_data<Api>(
    bot: &AutoSend<Throttle<Bot>>,
    request_model: &RequestModel,
    n: u8,
) -> Result<(), anyhow::Error>
where
    Api: ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
    <Api as ApiContentReceiver>::Out: ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
{
    let api = Api::from_env();
    let user_info = api.get_user_info(&request_model.user).await?;
    if let Some(user_info) = user_info {
        let mut data = api
            .get_content(user_info.id(), n, request_model.stype)
            .await?;
        data.truncate(n as usize);
        super::download_content(ContentForDownload::Array(&data)).await;
        for i in data.into_iter().rev() {
            super::send_content(
                &api,
                bot,
                &user_info,
                &request_model.chat_id,
                &i,
                OutputType::BySubscription(request_model.stype),
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
    bot: &AutoSend<Throttle<Bot>>,
    db: &MongoDatabase,
    model: &RequestModel,
) -> Result<(), anyhow::Error>
where
    Api: DatabaseInfoProvider
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + FromEnv<Api>
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
                let content = api.get_content(user_info.id(), 5, model.stype).await?;
                for item in content {
                    db.add_content::<Api>(item.id(), &model.user, model.stype)
                        .await?;
                }
            }
            db.subscribe_user::<Api>(user_info.unique_user_name(), &model.chat_id, model.stype)
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
    bot: &AutoSend<Throttle<Bot>>,
    api: &Api,
    db: &MongoDatabase,
    user: &User,
    stype: SubscriptionType,
) -> Result<(), anyhow::Error>
where
    <Api as ApiContentReceiver>::Out: GetId + ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    Api: DatabaseInfoProvider
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
{
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
        let content = api.get_content(user_info.id(), 5, stype).await?;
        log::info!("{}: Received user {} data", Api::name(), &user.id);
        let content = filter_sent_videos::<Api, <Api as ApiContentReceiver>::Out>(
            db, content, &user.id, stype,
        )
        .await;
        download_content(ContentForDownload::Array(&content)).await;
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
                >(
                    api,
                    bot,
                    &user_info,
                    chat,
                    &element,
                    OutputType::BySubscription(stype),
                )
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
    bot: &AutoSend<Throttle<Bot>>,
    api: &Api,
    db: &MongoDatabase,
) -> Result<(), anyhow::Error>
where
    <Api as ApiContentReceiver>::Out: GetId + ReturnDataForDownload + ReturnTextInfo,
    <Api as ApiUserInfoReceiver>::Out: ReturnUserInfo,
    Api: DatabaseInfoProvider
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateMessage<<Api as ApiUserInfoReceiver>::Out, <Api as ApiContentReceiver>::Out>,
{
    let users = db.get_collection::<Api, crate::database::User>().await?;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
    for user in users {
        for stype in SubscriptionType::iterator() {
            interval.tick().await;
            process_user::<Api>(bot, api, db, &user, stype)
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
        db.is_content_showed::<Api>(elem.id(), user_id, stype)
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
