use anyhow::{Context, Result};
use mongodb::bson::{self, doc};
use poise::{ChoiceParameter, CreateReply};
use serde::{Deserialize, Serialize};

use super::CommandContext;
use crate::{
    models::{AutoproxySettings, DBUserSettings, Latch},
    utils::user_settings::{get_or_create_user_settings, update_user_settings},
};

#[poise::command(slash_command, subcommands("autoproxy"))]
pub async fn settings(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

#[derive(Clone, Debug, Serialize, Deserialize, ChoiceParameter, PartialEq)]
pub enum AutoproxySlashOptions {
    #[name = "Do not autoproxy at all"]
    Disabled,
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
                AutoproxySlashOptions::Disabled => AutoproxySettings::Disabled,
                AutoproxySlashOptions::SwitchedIn => AutoproxySettings::SwitchedIn,
                AutoproxySlashOptions::Latch => AutoproxySettings::Latch(Latch::Global(None)),
            }
        }
    }
}

/// Changes your autoproxy settings
#[poise::command(slash_command, ephemeral)]
pub async fn autoproxy(
    ctx: CommandContext<'_>,
    autoproxy: Option<AutoproxySlashOptions>,
    #[description = "If specified, automatically proxy as this mate"] mate: Option<String>,
    #[description = "If true, specifies autoproxy settings only for this guild"] guild_only: Option<
        bool,
    >,
    #[description = "If true, makes the autoproxy settings for this guild revert to the global \
                     settings"]
    revert: Option<bool>,
) -> Result<()> {
    let database = &ctx.data().database;

    let settings_collection = database.collection::<DBUserSettings>("settings");

    let settings = get_or_create_user_settings(
        &settings_collection,
        ctx.author().id,
        if guild_only == Some(true) || revert == Some(true) {
            Some(
                ctx.guild_id()
                    .context("You cannot set autoproxy settings for a guild in DMs.")?
                    .get() as i64,
            )
        } else {
            None
        },
    )
    .await?;

    if revert == Some(true) {
        settings_collection
            .update_one(
                doc! {
                    "user_id": settings.user_id as i64,
                    "guild_id": settings.guild_id
                },
                doc! { "$set": { "autoproxy": bson::to_bson(&None::<AutoproxySettings>).unwrap() } },
                None,
            )
            .await?;

        ctx.send(CreateReply::new().content("Successfully updated your autoproxy settings!"))
            .await?;

        return Ok(());
    }

    let mut new_autoproxy;

    new_autoproxy = autoproxy
        .context("You need to specify an autoproxy setting!")?
        .into_dbsettings(mate);

    match new_autoproxy {
        AutoproxySettings::Latch(_) => {
            if guild_only == Some(true) {
                new_autoproxy = AutoproxySettings::Latch(Latch::Guild(None))
            } else {
                new_autoproxy = AutoproxySettings::Latch(Latch::Global(None))
            }
        }
        _ => {}
    }

    update_user_settings(&settings_collection, settings, Some(new_autoproxy)).await?;

    ctx.send(CreateReply::new().content("Successfully updated your autoproxy settings!"))
        .await?;

    Ok(())
}
