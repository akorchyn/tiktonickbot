use crate::api::*;

use teloxide::prelude::*;

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use crate::database::User;
use futures::future::join_all;

use super::*;

pub(crate) async fn run(bot: AutoSend<Bot>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
    let tiktok_api = TiktokApi::from_env();
    let twitter_api = TwitterApi::from_env();

    loop {
        interval.tick().await;
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
    log::info!("{}: User {} processing started.", Api::name(), &user.id);
    let content = api.get_content(&user_info.id(), 5, stype).await?;
    log::info!("{}: Received user {} data", Api::name(), &user.id);
    let content =
        filter_sent_videos::<Api, <Api as ApiContentReceiver>::Out>(db, content, &user.id, stype)
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
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

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
