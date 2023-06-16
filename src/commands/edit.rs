use std::num::NonZeroU64;

use anyhow::{Context, Result};
use mongodb::bson::doc;
use poise::serenity_prelude::{self as serenity, CacheHttp, EditWebhookMessage, MessageId};

use super::{autocomplete::mate as mate_autocomplete, CommandContext};
use crate::{
    models::{DBChannel, DBCollective, DBMate, DBMessage},
    utils,
};

#[poise::command(slash_command, subcommands("mate", "collective", "message"))]
pub async fn edit(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// Edit a mate
#[poise::command(slash_command, ephemeral)]
pub async fn mate(
    ctx: CommandContext<'_>,
    #[description = "the current name of the mate"]
    #[autocomplete = "mate_autocomplete"]
    name: String,
    #[description = "the new trigger for proxying (ie `[text]`)"] selector: Option<String>,
    #[description = "the new name to show in chat when proxying"] display_name: Option<String>,
    #[description = "whether to allow other people to use /info for this mate"] publicity: Option<
        bool,
    >,
    #[description = "the new avatar to use when proxying"] avatar: Option<serenity::Attachment>,
    #[description = "the mate's bio"] bio: Option<String>,
    #[description = "the mate's pronouns"] pronouns: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    utils::get_mate(&mates_collection, ctx.author().id, name.clone())
        .await
        .context("Failed to find mate to edit; does it exist?")?
        .edit(
            ctx.http(),
            mates_collection,
            None,
            display_name,
            bio,
            pronouns,
            selector,
            publicity,
            avatar,
        )
        .await?;

    ctx.say("Successfully edited mate!").await?;

    Ok(())
}

/// Edit details about your collective (shown on /info)
#[poise::command(slash_command, ephemeral)]
pub async fn collective(
    ctx: CommandContext<'_>,
    #[description = "the name of your collective"] name: Option<String>,
    #[description = "the bio of your collective"] bio: Option<String>,
    #[description = "whether your collective should be viewable by others with /info"]
    publicity: Option<bool>,
    #[description = "the collective pronouns of your collective"] pronouns: Option<String>,
    #[description = "A tag to append to all proxied mates, to identify your collective in chat"]
    collective_tag: Option<String>,
    #[description = "If true, remove your collective tag"] remove_collective_tag: Option<bool>,
) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");

    utils::get_or_create_collective(&collectives_collection, ctx.author().id)
        .await?
        .edit(
            collectives_collection,
            name,
            bio,
            pronouns,
            publicity,
            if let Some(true) = remove_collective_tag {
                Some("".to_string())
            } else {
                collective_tag
            },
        )
        .await?;

    ctx.say("Successfully updated your collective!").await?;

    Ok(())
}

/// Edit a proxied message in the current channel (if none is specified, edit the most recent one)
#[poise::command(slash_command, ephemeral)]
pub async fn message(
    ctx: CommandContext<'_>,
    #[description = "The new message content"] content: String,
    #[description = "The raw ID of the message to edit"] message_id: Option<u64>,
    #[description = "A link to the message to edit"] message_link: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let channels_collection = database.collection::<DBChannel>("channels");
    let messages_collection = database.collection::<DBMessage>("messages");

    let message_to_edit_id;
    if let Some(message_id) = message_id {
        message_to_edit_id = MessageId(NonZeroU64::new(message_id).unwrap())
    } else if let Some(message_link) = message_link {
        message_to_edit_id = utils::message_link_to_id(message_link)?
    } else {
        let message = utils::get_most_recent_message(&messages_collection, ctx.author().id).await?;
        message_to_edit_id = MessageId(NonZeroU64::new(message.message_id).unwrap())
    }

    let (webhook, thread_id) =
        utils::get_webhook_or_create(ctx.http(), &channels_collection, ctx.channel_id()).await?;

    _ = utils::get_message(
        &messages_collection,
        Some(ctx.author().id),
        message_to_edit_id,
    )
    .await?;

    let mut builder = EditWebhookMessage::new().content(content);

    if let Some(thread_id) = thread_id {
        builder = builder.in_thread(thread_id)
    }

    webhook
        .edit_message(ctx.http(), message_to_edit_id, builder)
        .await?;

    ctx.say("Edited message!").await?;

    Ok(())
}
