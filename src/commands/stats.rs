use crate::models::DBMate;

use super::CommandContext;
use anyhow::{Context, Result};
use mongodb::bson::doc;
use poise::serenity_prelude::CacheHttp;

/// Get the statistics of the bot
#[poise::command(slash_command)]
pub async fn stats(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    let cache = ctx
        .cache()
        .context("Failed to get bot cache; try again later!")?;

    let user_count = mates_collection
        .distinct("user_id", None, None)
        .await?
        .len();
    let guild_count = cache.guild_count();

    ctx.say(format!(
        "Serving **{}** users and **{}** guilds!",
        user_count, guild_count,
    ))
    .await?;

    Ok(())
}

/// Ping pong 🏓
#[poise::command(slash_command)]
pub async fn ping(ctx: CommandContext<'_>) -> Result<()> {
    ctx.say("Pong :3").await?;

    Ok(())
}
