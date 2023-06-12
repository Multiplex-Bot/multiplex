use super::CommandContext;
use anyhow::Result;
use mongodb::bson::doc;

#[poise::command(slash_command, subcommands("sticky"))]
pub async fn settings(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// Whether a bracket-based proxy should automatically switch to the proxier
#[poise::command(slash_command, ephemeral)]
pub async fn sticky(_ctx: CommandContext<'_>, _enabled: bool) -> Result<()> {
    Ok(())
}
