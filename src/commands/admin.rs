use std::num::NonZeroU64;

use anyhow::{Context, Result};
use mongodb::bson::doc;
use poise::serenity_prelude::{self as serenity, CacheHttp, MessageId};

use super::{autocomplete::mate as mate_autocomplete, CommandContext};
use crate::{
    models::{DBChannel, DBMate, DBMessage},
    utils,
};

#[poise::command(slash_command, subcommands("proxy_logs"))]
pub async fn admin(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

#[poise::command(slash_command, ephemeral, required_permissions = "MANAGE_GUILD")]
pub async fn proxy_logs(ctx: CommandContext<'_>) -> Result<()> {
    Ok(())
}
