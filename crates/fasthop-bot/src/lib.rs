use anyhow::Result;
use fasthop_shared::{db, models::Subscription};
use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "FastHop commands:")]
enum Command {
    #[command(description = "create or show your DNS link")]
    Start,
    #[command(description = "show access status")]
    Status,
    #[command(description = "show setup instructions")]
    Setup,
    #[command(description = "show your DNS link")]
    Link,
    #[command(description = "show help")]
    Help,
}

pub async fn spawn(token: String, pool: PgPool, base_url: String, trial_hours: i64) -> Result<()> {
    let bot = Bot::new(token);
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, message: Message, command: Command| {
            let pool = pool.clone();
            let base_url = base_url.clone();
            async move {
                let id = message.from.as_ref().map(|user| user.id.0 as i64);
                let text = match command {
                    Command::Start => {
                        let Some(id) = id else { return respond(()) };
                        match db::get_subscription(&pool, id, trial_hours).await {
                            Ok(subscription) => format!(
                                "Welcome to FastHop.\nYour DoH URL:\n{}/b/{}",
                                base_url, subscription.token
                            ),
                            Err(_) => "Unable to create your subscription right now.".to_owned(),
                        }
                    }
                    Command::Status => {
                        let Some(id) = id else { return respond(()) };
                        match db::get_subscription(&pool, id, trial_hours).await {
                            Ok(subscription) => format!(
                                "Status: {:?}\nExpires: {}",
                                subscription.status, subscription.expires_at
                            ),
                            Err(_) => "Send /start first.".to_owned(),
                        }
                    }
                    Command::Setup => {
                        "Configure your DNS app with the private DoH URL from /start.".to_owned()
                    }
                    Command::Link => {
                        let Some(id) = id else { return respond(()) };
                        match db::get_subscription(&pool, id, trial_hours).await {
                            Ok(subscription) => format!("{}/b/{}", base_url, subscription.token),
                            Err(_) => "Send /start first.".to_owned(),
                        }
                    }
                    Command::Help => "Use /start, /status, /setup, or /link.".to_owned(),
                };
                bot.send_message(message.chat.id, text).await?;
                respond(())
            }
        });
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

pub fn user_label(subscription: &Subscription) -> String {
    subscription.telegram_id.to_string()
}
