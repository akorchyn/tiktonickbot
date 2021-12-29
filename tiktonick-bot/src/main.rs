use std::env;
use std::fs;

use teloxide::prelude::*;

mod api;
mod database;
mod processing;

use std::sync::mpsc::sync_channel;

#[tokio::main]
async fn main() {
    std::env::var("TELEGRAM_ADMIN_ID").expect("Expect admin id.");
    teloxide::enable_logging!();
    if let Err(e) = fs::create_dir_all("content") {
        log::error!("Error: couldn't create videos directory.\n{}", e);
        return;
    }
    let (sender, receiver) = sync_channel::<processing::UserRequest>(5000);
    let bot = Bot::from_env().auto_send();
    tokio::spawn(processing::updater::run(bot.clone(), receiver));
    processing::bot::run(bot, sender).await;
}
