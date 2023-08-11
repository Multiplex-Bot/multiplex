use anyhow::Result;
use poise::serenity_prelude::{self as serenity};

use super::CommandContext;
use crate::{models::DBGuild, utils};

// if required_permissions ever doesn't work or breaks we are FUCKED (capital-er f)

#[poise::command(slash_command, subcommands("proxy_logs"))]
pub async fn admin(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

#[poise::command(slash_command, ephemeral, required_permissions = "MANAGE_GUILD")]
pub async fn proxy_logs(
    ctx: CommandContext<'_>,
    #[description = "the channel to send proxy logs to (reests to none, if unspecified!)"] channel: Option<serenity::Channel>,
) -> Result<()> {
    let database = &ctx.data().database;

    let guilds_collection = database.collection::<DBGuild>("guilds");

    let guild = utils::get_or_create_dbguild(
        &guilds_collection,
        ctx.guild_id()
            .expect("Couldn't get the guild id! Are you running this command in DMs?")
            .get() as i64,
    )
    .await?;

    if let Some(channel) = channel {
        let channel = channel.id().get() as i64;

        utils::update_guild_settings(&guilds_collection, guild, Some(channel)).await?;

        ctx.say(format!("Set proxy logging channel to <#{}>", channel))
            .await?;
    } else {
        utils::update_guild_settings(&guilds_collection, guild, None).await?;

        ctx.say("Disabled proxy logging!").await?;
    }

    Ok(())
}
