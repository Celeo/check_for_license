use anyhow::Result;
use std::env;

mod models;
use models::Config;
mod bot;
use bot::Bot;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    kankyo::init().expect("Could not load .env file");
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "check_for_license");
    }
    pretty_env_logger::init();

    let mut bot = Bot::new(Config::from_env()?)?;
    bot.login().await?;

    bot.watch_subreddit("celeo").await?;

    Ok(())
}
