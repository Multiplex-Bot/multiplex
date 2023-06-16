use anyhow::Result;

use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, CreateMessage, Reaction};

use crate::commands::Data;
use crate::models::{DBChannel, DBMessage};
use crate::utils::{self, get_webhook_or_create};

pub async fn run(ctx: &SerenityContext, data: &Data, reaction: &Reaction) -> Result<()> {
    let database = &data.database;
    let messages_collection = database.collection::<DBMessage>("messages");
    let channels_collection = database.collection::<DBChannel>("channels");

    let original_message =
        utils::get_message(&messages_collection, None, reaction.message_id).await?;

    let (webhook, thread_id) =
        get_webhook_or_create(ctx.http(), &channels_collection, reaction.channel_id).await?;

    if reaction.emoji.unicode_eq("❌") {
        if original_message.user_id == reaction.user_id.unwrap().0.get() {
            webhook
                .delete_message(ctx.http(), thread_id, reaction.message_id)
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

    Ok(())
}
