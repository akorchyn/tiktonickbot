use crate::api::{ApiContentReceiver, ApiUserInfoReceiver};
use crate::database::tiktok::DatabaseApi;

use teloxide::prelude::*;

use futures::future::join_all;

use super::*;

pub(crate) async fn run(bot: AutoSend<Bot>) {
    let db = create_db()
        .await
        .expect("Expected successful connection to DB");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));

    loop {
        interval.tick().await;
        log::info!("Started updating TikTok feeds");
        tiktok_updates_monitor_run(&bot, &db)
            .await
            .unwrap_or_else(|e| {
                log::error!("Tiktok update run failed with an error: {}", e);
            });
        log::info!("Finished updating TikTok feeds");
    }
}

async fn tiktok_updates_monitor_run(
    bot: &AutoSend<Bot>,
    db: &MongoDatabase,
) -> Result<(), anyhow::Error> {
    let users = db.get_users::<tiktokdb::User>().await?;
    let api = tiktokapi::TiktokApi::from_env();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

    for user in users {
        interval.tick().await;
        for stype in tiktokdb::SubscriptionType::iterator() {
            let chats = user.get_chats_by_subscription_type(stype);
            if chats.is_none() {
                continue;
            }
            let chats = chats.as_ref().unwrap();
            log::info!("User {} processing started.", &user.tiktok_username);
            let videos = get_videos_to_send(&db, &user.tiktok_username, stype).await?;
            download_videos(&videos).await;
            for video in videos {
                for chat in chats {
                    log::info!(
                        "Sending video from {} to chat {}",
                        user.tiktok_username,
                        chat
                    );
                    let tiktok_info = api.get_user_info(&user.tiktok_username).await?;
                    match send_video(&bot, &tiktok_info, chat, &video, stype).await {
                        Ok(_) => {
                            db.add_content(&video.id, &user.tiktok_username, stype)
                                .await?
                        }
                        Err(e) => log::error!(
                            "Error happened during sending video to {} with below error:\n{}",
                            &chat,
                            e
                        ),
                    }
                }
            }
            log::info!("User {} processing finished.", &user.tiktok_username);
        }
    }
    Ok(())
}

async fn filter_sent_videos(
    db: &MongoDatabase,
    videos: Vec<tiktokapi::Video>,
    username: &str,
    stype: tiktokdb::SubscriptionType,
) -> Vec<tiktokapi::Video> {
    let to_remove: Vec<_> = join_all(videos.iter().map(|video| async {
        db.is_video_showed(&video.id, &username, stype)
            .await
            .unwrap_or(true)
    }))
    .await;

    videos
        .into_iter()
        .zip(to_remove.into_iter())
        .filter(|(_, f)| !*f)
        .map(|(v, _)| v)
        .collect::<Vec<_>>()
}

async fn get_videos_to_send(
    db: &MongoDatabase,
    username: &str,
    stype: tiktokdb::SubscriptionType,
) -> Result<Vec<tiktokapi::Video>, anyhow::Error> {
    let api = tiktokapi::TiktokApi::from_env();
    let likes = api.get_content(&username, 5, stype).await?;
    log::info!("Received user {} likes", &username);
    Ok(filter_sent_videos(db, likes, &username, stype).await)
}
