mod commands;
mod events;
mod models;
mod pluralkit;
mod tupperbox;
mod utils;

use std::{env, num::NonZeroU64, time::Duration};

use axum::{routing::get, Router};
use commands::Data;
use dotenvy::dotenv;
use mongodb::{options::ClientOptions, Client as MongoClient};
use poise::{
    serenity_prelude::{CacheHttp, Client, Command, FullEvent, GatewayIntents, GuildId},
    Framework,
};
use s3::{creds::Credentials, region::Region, Bucket};
use tokio::{task::JoinSet, time::sleep};

use crate::utils::misc::envvar;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv()
        .expect("Could not find environment config; did you forget to `cp .env.template .env`?");

    let mut client_options = ClientOptions::parse(envvar("DATABASE_URL"))
        .await
        .expect("Failed to create MongoDB client options! (Somehow)");

    client_options.app_name = Some("Multiplex".to_string());

    let client =
        MongoClient::with_options(client_options).expect("Failed to open MongoDB connection!");

    let db = client.database(&envvar("DATABASE_NAME"));

    let avatar_bucket = Bucket::new(
        &envvar("S3_AVATAR_BUCKET"),
        Region::Custom {
            region: envvar("S3_REGION"),
            endpoint: envvar("S3_ENDPOINT"),
        },
        Credentials::new(
            Some(&envvar("S3_KEY_ID")),
            Some(&envvar("S3_KEY_SECRET")),
            None,
            None,
            None,
        )
        .unwrap(),
    )
    .unwrap()
    .with_path_style();

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::misc::explain(),
            commands::misc::ping(),
            commands::misc::support(),
            commands::misc::stats(),
            commands::mate::create(),
            commands::delete::delete(),
            commands::mate::switch(),
            commands::edit::edit(),
            commands::info::info(),
            commands::import::import(),
            commands::export::export(),
            commands::settings::settings(),
            commands::admin::admin(),
            commands::switch_logs::switch_logs(),
        ],
        listener: |event, _framework, data| {
            Box::pin(async move {
                match event {
                    FullEvent::Ready {
                        ctx: _,
                        data_about_bot: _,
                    } => {
                        tracing::info!("Bot is ready!")
                    }
                    FullEvent::Message { ctx, new_message } => {
                        if new_message.content.starts_with(&envvar("PREFIX")) {
                            events::on_text_command::run(ctx, data, new_message).await?
                        } else {
                            events::on_message::run(ctx, data, new_message).await?
                        }
                    }
                    FullEvent::MessageUpdate {
                        ctx,
                        old_if_available: _,
                        new: _,
                        event,
                    } => events::on_edit::run(ctx, data, event).await?,
                    FullEvent::ReactionAdd { ctx, add_reaction } => {
                        events::on_reaction::run(ctx, data, add_reaction).await?
                    }
                    _ => {}
                }

                Ok(())
            })
        },
        ..Default::default()
    };

    let mut client = Client::builder(
        envvar("TOKEN"),
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(Framework::new(options, |ctx, _ready, framework| {
        Box::pin(async move {
            let create_commands =
                poise::builtins::create_application_commands(&framework.options().commands);
            if let Ok(id) = env::var("DEV_GUILD") {
                GuildId(NonZeroU64::new(id.parse::<u64>().unwrap()).unwrap())
                    .set_commands(ctx.http(), create_commands)
                    .await?;
                tracing::info!("Using guild-specific slash commands in {}", id);
            } else {
                Command::set_global_commands(ctx.http(), create_commands).await?;
                tracing::info!(
                    "Using global slash commands; warning, this may take literally forever"
                );
            }
            Ok(Data {
                database: db,
                avatar_bucket,
            })
        })
    }))
    .await
    .unwrap();

    let mut threads = JoinSet::new();

    threads.spawn(async move {
        client.start().await.unwrap();
    });

    threads.spawn(async move {
        let app = Router::new().route("/health", get(|| async { "( •̀ ω •́ )✧" }));

        axum::Server::bind(&envvar("HEALTH_CHECK_ADDRESS").parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    while !threads.is_empty() {
        sleep(Duration::from_secs(10)).await;
    }
}
