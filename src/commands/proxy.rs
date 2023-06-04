use crate::models::{DBChannel, DBCollective, DBMate, DBMessage};

use super::CommandContext;
use anyhow::{Context, Result};
use mongodb::{bson::doc, options::FindOneOptions, Database};
use poise::serenity_prelude::{
    self as serenity, CacheHttp, ExecuteWebhook, MessageId, Webhook, WebhookId,
};

#[poise::command(slash_command, subcommands("message", "embed"))]
pub async fn proxy(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

async fn send_proxied_message(
    ctx: CommandContext<'_>,
    message: &ExecuteWebhook<'_>,
    channel: DBChannel,
    mate: DBMate,
    collective: DBCollective,
    database: &Database,
) {
}

#[poise::command(slash_command)]
pub async fn message(
    ctx: CommandContext<'_>,
    #[description = "The name of the mate to proxy as"] mate: String,
    #[description = "The content of the proxied message"] content: String,
) -> Result<()> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn embed(
    ctx: CommandContext<'_>,
    #[description = "The name of the mate to proxy as"] mate: String,
) -> Result<()> {
    Ok(())
}
