use std::num::NonZeroU64;

use crate::models::{DBChannel, DBMate, DBMessage};

use super::autocomplete::mate as mate_autocomplete;
use super::CommandContext;
use anyhow::{Context, Result};
use mongodb::{bson::doc, options::FindOneOptions};
use poise::serenity_prelude::{CacheHttp, MessageId, Webhook, WebhookId};

#[poise::command(slash_command, subcommands("mate", "message"))]
pub async fn delete(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// Delete a mate
#[poise::command(slash_command)]
pub async fn mate(
    ctx: CommandContext<'_>,
    #[description = "name of the mate to delete"]
    #[autocomplete = "mate_autocomplete"]
    name: String,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    let old_mate = mates_collection
        .find_one(
            doc! { "user_id": ctx.author().id.0.get() as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(_)) = old_mate {
        mates_collection
            .delete_one(
                doc! { "user_id": ctx.author().id.0.get() as i64, "name": name.clone() },
                None,
            )
            .await?;
    } else {
        return Err(anyhow::anyhow!("Can't delete a mate that doesn't exist!"));
    }
    ctx.say("Successfully deleted mate! o7 :headstone:").await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn message(
    ctx: CommandContext<'_>,
    #[description = "The raw ID of the message to edit"] message_id: Option<String>,
    #[description = "A link to the message to edit"] message_link: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let channels_collection = database.collection::<DBChannel>("channels");
    let messages_collection = database.collection::<DBMessage>("messages");

    let channel = channels_collection
        .find_one(doc! {"id": ctx.channel_id().0.get() as i64}, None)
        .await?
        .context("oopsie daisy")?;

    let message_to_delete_id;
    if let Some(message_id) = message_id {
        message_to_delete_id = MessageId(NonZeroU64::new(message_id.parse::<u64>()?).unwrap())
    } else if let Some(message_link) = message_link {
        // https://discord.com/channels/891039687785996328/1008966348862390312/1109539731655626763 -> 1109539731655626763
        let iter = message_link.split("/");
        let message_id = iter.last().context("Failed to get message ID from link!")?;
        message_to_delete_id = MessageId(NonZeroU64::new(message_id.parse::<u64>()?).unwrap())
    } else {
        let message = messages_collection
            .find_one(
                doc! { "user_id": ctx.author().id.0.get() as i64 },
                Some(FindOneOptions::builder().sort(doc! {"_id": -1}).build()),
            )
            .await?
            .context("Failed to get most recent message!")?;
        message_to_delete_id = MessageId(NonZeroU64::new(message.message_id).unwrap())
    }

    messages_collection
        .find_one(
            doc! { "message_id": i64::from(message_to_delete_id), "user_id": ctx.author().id.0.get() as i64 },
            None,
        )
        .await?
        .context("Sorry, the proxied message is not available; leave a message at the tone.")?;

    let webhook = Webhook::from_id_with_token(
        ctx.http(),
        WebhookId(NonZeroU64::new(channel.webhook_id as u64).unwrap()),
        &channel.webhook_token,
    )
    .await?;

    webhook
        .delete_message(ctx.http(), message_to_delete_id)
        .await?;

    ctx.say("Deleted message! o7 :headstone:").await?;
    Ok(())
}
