use anyhow::Result;
use mongodb::bson::{self, doc};
use poise::serenity_prelude::{self as serenity};

use super::CommandContext;
use crate::{
    models::DBGuild,
    utils::{guild_settings::update_guild_settings, guilds::get_or_create_dbguild},
};

// if required_permissions ever doesn't work or breaks we are FUCKED (capital-er f)

#[poise::command(slash_command, subcommands("proxy_logs", "allowlist"))]
pub async fn admin(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// If set, allow only this role to use the bot
#[poise::command(slash_command, ephemeral, required_permissions = "MANAGE_GUILD")]
pub async fn allowlist(
    ctx: CommandContext<'_>,
    #[description = "the allowlisting role (resets to none, if unspecified)"] role: Option<
        serenity::Role,
    >,
) -> Result<()> {
    let database = &ctx.data().database;

    let guilds_collection = database.collection::<DBGuild>("guilds");

    let mut guild = get_or_create_dbguild(
        &guilds_collection,
        ctx.guild_id()
            .expect("Couldn't get the guild id! Are you running this command in DMs?")
            .get() as i64,
    )
    .await?;

    if let Some(role) = role.clone() {
        let role_id = role.id.get() as i64;

        guild.allowlist_role = Some(role_id);
    } else {
        guild.allowlist_role = None;
    }

    guilds_collection
        .update_one(
            doc! {
                "id": guild.id
            },
            doc! { "$set": bson::to_bson(&guild).unwrap() },
            None,
        )
        .await?;

    if let Some(role) = role {
        ctx.say(format!("Set allowlist role to <@&{}>", role.id))
            .await?;
    } else {
        ctx.say("Disabled allowlist!").await?;
    }

    Ok(())
}

#[poise::command(slash_command, ephemeral, required_permissions = "MANAGE_GUILD")]
pub async fn proxy_logs(
    ctx: CommandContext<'_>,
    #[description = "the channel to send proxy logs to (resets to none, if unspecified!)"] channel: Option<serenity::Channel>,
) -> Result<()> {
    let database = &ctx.data().database;

    let guilds_collection = database.collection::<DBGuild>("guilds");

    let guild = get_or_create_dbguild(
        &guilds_collection,
        ctx.guild_id()
            .expect("Couldn't get the guild id! Are you running this command in DMs?")
            .get() as i64,
    )
    .await?;

    if let Some(channel) = channel {
        let channel = channel.id().get() as i64;

        update_guild_settings(&guilds_collection, guild, Some(channel)).await?;

        ctx.say(format!("Set proxy logging channel to <#{}>", channel))
            .await?;
    } else {
        update_guild_settings(&guilds_collection, guild, None).await?;

        ctx.say("Disabled proxy logging!").await?;
    }

    Ok(())
}
