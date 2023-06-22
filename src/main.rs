mod commands;
mod events;
mod models;
mod pluralkit;
mod tupperbox;
mod utils;

use std::{env, num::NonZeroU64};

use anyhow::Context;
use commands::Data;
use dotenvy::dotenv;
use mongodb::{options::ClientOptions, Client as MongoClient};
use poise::{
    serenity_prelude::{CacheHttp, Client, Command, FullEvent, GatewayIntents, GuildId},
    Framework,
};
use s3::{creds::Credentials, region::Region, Bucket};

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

    let client =
        MongoClient::with_options(client_options).expect("Failed to open MongoDB connection!");

    let db = client.database(
        &env::var("DATABASE_NAME")
            .expect("Could not find database name; did you specify it in .env?"),
    );

    let avatar_bucket = Bucket::new(
        &env::var("S3_AVATAR_BUCKET")
            .expect("Could not find avatar bucket name; did you specify it in .env?"),
        Region::Custom {
            region: env::var("S3_REGION")
                .expect("Could not find S3 region; did you specify it in .env?"),
            endpoint: env::var("S3_ENDPOINT")
                .expect("Could not find S3 endpoint; did you specify it in .env?"),
        },
        Credentials::new(
            Some(&env::var("S3_KEY_ID").unwrap()),
            Some(&env::var("S3_KEY_SECRET").unwrap()),
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
            commands::misc::stats(),
            commands::mate::create(),
            commands::delete::delete(),
            commands::mate::switch(),
            commands::edit::edit(),
            commands::info::info(),
            commands::import::import(),
            commands::export::export(),
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
                        if new_message.content.starts_with(&env::var("PREFIX").expect(
                            "Could not find text command prefix; did you specify it in .env?",
                        )) {
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
        env::var("TOKEN").expect("$TOKEN not found; did you specify it in .env?"),
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

    client.start().await.unwrap();
}
