use crate::api::*;

use teloxide::prelude::*;

use crate::api::tiktok::TiktokApi;
use crate::api::twitter::TwitterApi;
use futures::future::join_all;

use super::*;

pub(crate) async fn run(bot: AutoSend<Bot>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
    let tiktok_api = TiktokApi::from_env();
    let twitter_api = TwitterApi::from_env();
    let admin_id: i64 = std::env::var("TELEGRAM_ADMIN_ID").unwrap().parse().unwrap();

    loop {
        interval.tick().await;
        log::info!("Started updating Twitter feeds");
        updates_monitor_run(&bot, &twitter_api, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("Twitter update run failed with an error: {}", e);
            });
        log::info!("Finished updating Twitter feeds");
        run_tiktok_api(&bot, &tiktok_api, &db, admin_id)
            .await
            .unwrap_or_else(|e| {
                log::error!("Twitter update run failed with an error: {}", e);
            });
    }
}

async fn run_tiktok_api(
    bot: &AutoSend<Bot>,
    api: &TiktokApi,
    db: &MongoDatabase,
    admin_id: i64,
) -> Result<(), anyhow::Error> {
    if api.check_alive().await {
        log::info!("Started updating TikTok feeds");
        updates_monitor_run::<TiktokApi>(&bot, &api, &db).await?;
        log::info!("Finished updating TikTok feeds");
    } else {
        log::info!("Tiktok api is dead");
        bot.send_message(
            admin_id,
            "Unfortunately, Tiktok api doesn't responds. Please, take care of it".to_string(),
        )
        .disable_notification(true)
        .await?;
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
        + ApiName
        + ApiContentReceiver
        + ApiUserInfoReceiver
        + GenerateSubscriptionMessage<
            <Api as ApiUserInfoReceiver>::Out,
            <Api as ApiContentReceiver>::Out,
        >,
{
    let users = db.get_collection::<Api, crate::database::User>().await?;

    for user in users {
        for stype in SubscriptionType::iterator() {
            let chats = user.get_chats_by_subscription_type(stype);
            if chats.is_none() {
                continue;
            }
            let chats = chats.as_ref().unwrap();
            if chats.is_empty() {
                continue;
            }
            let user_info = api.get_user_info(&user.id).await?;
            log::info!("{}: User {} processing started.", Api::name(), &user.id);
            let content = get_content_to_send::<Api>(&db, api, &user_info.id(), stype).await?;
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

async fn get_content_to_send<Api>(
    db: &MongoDatabase,
    api: &Api,
    username: &str,
    stype: SubscriptionType,
) -> Result<Vec<<Api as ApiContentReceiver>::Out>, anyhow::Error>
where
    Api: ApiContentReceiver + ApiName + DatabaseInfoProvider,
    <Api as ApiContentReceiver>::Out: GetId,
{
    let likes = api.get_content(&username, 5, stype).await?;
    log::info!("{}: Received user {} data", Api::name(), &username);
    Ok(
        filter_sent_videos::<Api, <Api as ApiContentReceiver>::Out>(db, likes, &username, stype)
            .await,
    )
}
