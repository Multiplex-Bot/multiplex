use anyhow::Result;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{mate, CommandContext};
use crate::models::{AutoproxySettings, DBUserSettings};

#[poise::command(slash_command, subcommands("autoproxy"))]
pub async fn settings(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

#[derive(Clone, Debug, Serialize, Deserialize, poise::ChoiceParameter)]
pub enum AutoproxySlashOptions {
    #[name = "Do not autoproxy at all"]
    None,
    #[name = "Automatically proxy as the switched-in mate"]
    SwitchedIn,
    #[name = "Automatically proxy as the last manually-proxied mate"]
    Latch,
}

impl AutoproxySlashOptions {
    pub fn into_dbsettings(&self, mate_name: Option<String>) -> AutoproxySettings {
        if let Some(mate_name) = mate_name {
            AutoproxySettings::Mate(mate_name)
        } else {
            match self {
                AutoproxySlashOptions::None => AutoproxySettings::None,
                AutoproxySlashOptions::SwitchedIn => AutoproxySettings::SwitchedIn,
                AutoproxySlashOptions::Latch => AutoproxySettings::Latch,
            }
        }
    }
}

/// Changes your autoproxy settings
#[poise::command(slash_command, ephemeral)]
pub async fn autoproxy(
    ctx: CommandContext<'_>,
    autoproxy: AutoproxySlashOptions,
    #[description = "If specified, automatically proxy as this mate"] mate: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let settings_collection = database.collection::<DBUserSettings>("settings");
    Ok(())
}
