use std::num::NonZeroU64;

use anyhow::{Context, Result};
use mongodb::{bson::doc, options::FindOneOptions};
use poise::serenity_prelude::{
    CacheHttp, ChannelType, Context as SerenityContext, EditWebhookMessage, Message, MessageId,
    Webhook, WebhookId,
};

use crate::{
    commands::Data,
    models::{DBChannel, DBMessage},
    utils::misc::envvar,
};

pub async fn run(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    let database = &data.database;
    let messages_collection = database.collection::<DBMessage>("messages");
    let channels_collection = database.collection::<DBChannel>("channels");

    match message
        .content
        .strip_prefix(&envvar("PREFIX"))
        .unwrap()
        .split_ascii_whitespace()
        .next()
        .unwrap()
    {
        "edit" | "e" => {
            let message_id;
            if let Some(message_ref) = message.referenced_message.clone() {
                let message = messages_collection
                    .find_one(
                        doc! { "user_id": message.author.id.0.get() as i64, "message_id": message_ref.id.0.get() as i64 },
                        Some(FindOneOptions::builder().sort(doc! {"_id": -1}).build()),
                    ).await;
                if let Ok(Some(_)) = message {
                    message_id = message_ref.id
                } else {
                    return Err(anyhow::anyhow!("You don't own that message"));
                }
            } else {
                let message = messages_collection
                    .find_one(
                        doc! { "user_id": message.author.id.0.get() as i64 },
                        Some(FindOneOptions::builder().sort(doc! {"_id": -1}).build()),
                    )
                    .await?
                    .context("Failed to get most recent message!")?;
                message_id = MessageId(NonZeroU64::new(message.message_id).unwrap())
            }

            message.delete(ctx.http()).await?;

            let channel_id;
            let guild_channel = message
                .channel(ctx.http())
                .await
                .context("Failed to get message's channel")?
                .guild()
                .context("Failed to get channel's guild channel")?;

            if guild_channel.kind == ChannelType::PublicThread
                || guild_channel.kind == ChannelType::PrivateThread
            {
                channel_id = guild_channel.parent_id.unwrap();
            } else {
                channel_id = message.channel_id;
            }

            let channel = channels_collection
                .find_one(doc! {"id": channel_id.0.get() as i64}, None)
                .await?
                .context("oopsie daisy")?;

            let webhook = Webhook::from_id_with_token(
                ctx.http(),
                WebhookId(NonZeroU64::new(channel.webhook_id as u64).unwrap()),
                &channel.webhook_token,
            )
            .await?;

            webhook
                .edit_message(
                    ctx.http(),
                    message_id,
                    EditWebhookMessage::new().content(
                        message
                            .content
                            .strip_prefix(&format!("{}{}", envvar("PREFIX"), "edit"))
                            .unwrap(),
                    ),
                )
                .await?;
        }
        _ => {}
    }
    Ok(())
}
