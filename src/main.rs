mod commands;
mod event_handler;
mod models;
mod pluralkit;
mod tupperbox;

use dotenvy::dotenv;
use mongodb::{options::ClientOptions, Client};
use poise::{
    serenity_prelude::{self as serenity, CacheHttp, FullEvent, GuildId},
    PrefixFrameworkOptions,
};
use std::{env, num::NonZeroU64};

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

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::stats::ping(),
            commands::stats::stats(),
            commands::mate::create(),
            commands::delete::delete(),
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
        listener: |event, _framework, data| {
            Box::pin(async move {
                match event {
                    FullEvent::Ready {
                        ctx,
                        data_about_bot: _,
                    } => {
                        tracing::info!("Bot is ready!")
                    }
                    FullEvent::Message { ctx, new_message } => {
                        if new_message.content.starts_with(&env::var("PREFIX").expect(
                            "Could not find text command prefix; did you specify it in .env?",
                        )) {
                            event_handler::on_text_command(ctx, data, new_message).await?
                        } else {
                            event_handler::on_message(ctx, data, new_message).await?
                        }
                    }
                    FullEvent::MessageUpdate {
                        ctx,
                        old_if_available: _,
                        new: _,
                        event,
                    } => event_handler::on_edit(ctx, data, event).await?,
                    FullEvent::ReactionAdd { ctx, add_reaction } => {
                        event_handler::on_reaction(ctx, data, add_reaction).await?
                    }
                    _ => {}
                }

                Ok(())
            })
        },
        ..Default::default()
    };

    let mut client = serenity::Client::builder(
        env::var("TOKEN").expect("$TOKEN not found; did you specify it in .env?"),
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(poise::Framework::new(options, |ctx, _ready, framework| {
        Box::pin(async move {
            let create_commands =
                poise::builtins::create_application_commands(&framework.options().commands);
            if let Ok(id) = env::var("DEV_GUILD") {
                GuildId(NonZeroU64::new(id.parse::<u64>().unwrap()).unwrap())
                    .set_commands(ctx.http(), create_commands)
                    .await?;
                tracing::info!("Using guild-specific slash commands in {}", id);
            } else {
                serenity::Command::set_global_commands(ctx.http(), create_commands).await?;
                tracing::info!(
                    "Using global slash commands; warning, this may take literally forever"
                );
            }
            Ok(Data { database: db })
        })
    }))
    .await
    .unwrap();

    client.start_autosharded().await.unwrap();
}
