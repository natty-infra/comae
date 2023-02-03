use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude as sp;
use sea_orm::{ConnectOptions, DatabaseConnection};
use serenity::model::application::command::Command;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

mod commands;
mod youtube_uploads;

struct Data {
    set_up_commands: AtomicBool,
    loop_running: AtomicBool,
    debug_mode: bool,
    database: DatabaseConnection,
}

async fn register_commands<E>(
    http: Arc<sp::Http>,
    framework: &poise::FrameworkContext<'_, Data, E>,
) -> Result<(), serenity::Error> {
    if framework
        .user_data
        .set_up_commands
        .swap(true, Ordering::Relaxed)
    {
        return Ok(());
    }

    let commands = &framework.options().commands;
    let create_commands = poise::builtins::create_application_commands(commands);
    Command::set_global_application_commands(http, |b| {
        *b = create_commands;
        b
    })
    .await?;

    Ok(())
}

async fn start_event_loop<'a, E>(
    ctx: Arc<sp::Http>,
    framework: &poise::FrameworkContext<'_, Data, E>,
) -> Result<(), serenity::Error> {
    if framework
        .user_data
        .loop_running
        .swap(true, Ordering::Relaxed)
    {
        return Ok(());
    }

    event_loop_main(
        ctx,
        framework.user_data.database.clone(),
        framework.user_data.debug_mode,
    )
    .await;

    Ok(())
}

async fn event_loop_main<'a>(ctx: Arc<sp::Http>, database: DatabaseConnection, debug_mode: bool) {
    let checker = youtube_uploads::UploadChecker::new(debug_mode, database).await;

    tokio::spawn(async move {
        let ctx = ctx.clone();
        loop {
            if let Err(err) = checker.check(&ctx).await {
                error!("Failed to check for YouTube posts: {:?}", err);
            }

            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
}

fn handle_event<'a, E: From<serenity::Error>>(
    ctx: &'a sp::Context,
    event: &'a poise::Event<'a>,
    framework: poise::FrameworkContext<'a, Data, E>,
    _data: &'a Data,
) -> poise::BoxFuture<'a, Result<(), E>> {
    Box::pin(async move {
        match event {
            poise::Event::Ready { .. } => {
                register_commands(ctx.http.clone(), &framework).await?;
            }
            poise::Event::CacheReady { .. } => {
                start_event_loop(ctx.http.clone(), &framework).await?;
            }
            _ => {}
        }

        Ok(())
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    let db_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let debug_mode = env::var("BOT_TESTING_MODE")
        .map(|s| matches!(s.as_ref(), "yes" | "on" | "1" | "true"))
        .unwrap_or(false);

    let mut opt = ConnectOptions::new(db_url.to_owned());
    opt.max_connections(32).min_connections(8);

    let database = sea_orm::Database::connect(&db_url).await?;
    Migrator::up(&database, None).await?;

    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = sp::GatewayIntents::non_privileged() | sp::GatewayIntents::MESSAGE_CONTENT;
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::account_age(),
                commands::add_channel(),
                commands::list_channels(),
                commands::remove_channel(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(">".to_string()),
                edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600))),
                case_insensitive_commands: true,
                mention_as_prefix: true,
                ..Default::default()
            },
            event_handler: handle_event,
            ..Default::default()
        })
        .token(token)
        .intents(intents)
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    set_up_commands: false.into(),
                    loop_running: false.into(),
                    debug_mode,
                    database,
                })
            })
        });

    framework.run().await?;
    Ok(())
}
