use std::num::NonZeroU64;

use anyhow::{Context, Result};
use mongodb::bson::doc;

use poise::serenity_prelude::{
    CacheHttp, Context as SerenityContext, CreateMessage, Reaction, Webhook, WebhookId,
};

use crate::commands::Data;
use crate::models::{DBChannel, DBMessage};

pub async fn run(ctx: &SerenityContext, data: &Data, reaction: &Reaction) -> Result<()> {
    let database = &data.database;
    let messages_collection = database.collection::<DBMessage>("messages");
    let channels_collection = database.collection::<DBChannel>("channels");

    let original_message = messages_collection
        .find_one(
            doc! { "message_id": reaction.message_id.0.get() as i64 },
            None,
        )
        .await;

    if let Ok(Some(original_message)) = original_message {
        let dbchannel = channels_collection
            .find_one(doc! {"id": reaction.channel_id.0.get() as i64 }, None)
            .await?
            .context("Failed to get channel webhook")?;

        let webhook = Webhook::from_id_with_token(
            ctx.http(),
            WebhookId(NonZeroU64::new(dbchannel.webhook_id as u64).unwrap()),
            &dbchannel.webhook_token,
        )
        .await?;

        if reaction.emoji.unicode_eq("❌") {
            if original_message.user_id == reaction.user_id.unwrap().0.get() {
                // FIXME: pass in thread ID
                webhook
                    .delete_message(ctx.http(), None, reaction.message_id)
                    .await?;
            }
        } else if reaction.emoji.unicode_eq("❓") {
            reaction
                .user(ctx.http())
                .await?
                .direct_message(
                    ctx.http(),
                    CreateMessage::new()
                        .content(format!("Message sent by <@{}>", original_message.user_id)),
                )
                .await?;
            reaction.delete(ctx.http()).await?;
        }
    }

    Ok(())
}
