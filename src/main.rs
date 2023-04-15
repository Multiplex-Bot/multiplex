mod commands;
mod event_handler;
mod models;
mod pluralkit;
mod tupperbox;

use dotenvy::dotenv;
use mongodb::{options::ClientOptions, Client};
use poise::serenity_prelude::{self as serenity, CacheHttp, GuildId};
use std::env;

use commands::Data;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv()
        .expect("Could not find environment config; did you forget to `cp .env.template .env`?");

    let mut client_options = ClientOptions::parse(
        env::var("DATABASE_URL").expect("Cound not find database URL; did you specify it in .env?"),
    )
    .await
    .expect("Failed to create MongoDB client options! (Somehow)");

    client_options.app_name = Some("Multiplex".to_string());

    let client = Client::with_options(client_options).expect("Failed to open MongoDB connection!");

    let db = client.database(
        &env::var("DATABASE_NAME")
            .expect("Could not find database name; did you specify it in .env?"),
    );

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::stats::ping(),
                commands::stats::stats(),
                commands::mate::create(),
                commands::mate::delete(),
                commands::mate::switch(),
                commands::edit::edit(),
                commands::info::info(),
                commands::import::import(),
                commands::export::export(),
            ],
            pre_command: |ctx| {
                Box::pin(async move {
                    ctx.defer_ephemeral()
                        .await
                        .expect("Failed to make response ephemeral");
                })
            },
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    match event {
                        poise::Event::Ready { data_about_bot: _ } => {
                            tracing::info!("Bot is ready!")
                        }
                        poise::Event::Message { new_message } => {
                            event_handler::on_message(ctx, data, new_message).await?
                        }
                        _ => {}
                    }

                    Ok(())
                })
            },
            ..Default::default()
        })
        .token(env::var("TOKEN").expect("$TOKEN not found; did you specify it in .env?"))
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                if let Ok(id) = env::var("DEV_GUILD") {
                    poise::builtins::register_in_guild(
                        ctx.http(),
                        &framework.options().commands,
                        GuildId(id.parse::<u64>().unwrap()),
                    )
                    .await?;
                    tracing::info!("Using guild-specific slash commands in {}", id);
                } else {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    tracing::info!(
                        "Using global slash commands; warning, this may take literally forever"
                    );
                }
                Ok(Data { database: db })
            })
        });

    framework.build().await.unwrap().start_autosharded().await.unwrap();
}
