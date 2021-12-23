use std::env;
use std::fs;

use teloxide::prelude::*;

mod api;
mod database;
mod processing;

#[tokio::main]
async fn main() {
    std::env::var("TELEGRAM_ADMIN_ID").expect("Expect admin id.");
    teloxide::enable_logging!();
    if let Err(e) = fs::create_dir_all("content") {
        log::error!("Error: couldn't create videos directory.\n{}", e);
        return;
    }
    let bot = Bot::from_env().auto_send();
    let bot_name: String = String::from("Tikitoki Likes");
    tokio::spawn(processing::updater::run(bot.clone()));
    processing::bot::run(bot, bot_name).await;
}