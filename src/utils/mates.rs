use anyhow::{Context, Result};
use mongodb::{bson::doc, results::DeleteResult, Collection};
use poise::{
    futures_util::TryStreamExt,
    serenity_prelude::{GuildId, UserId},
};

use super::user_settings::get_or_create_user_settings;
use crate::models::{AutoproxySettings, DBMate, DBUserSettings, Latch};

pub async fn get_mate(
    collection: &Collection<DBMate>,
    user_id: UserId,
    name: String,
) -> Option<DBMate> {
    collection
        .find_one(doc! { "user_id": user_id.get() as i64, "name": name }, None)
        .await
        .ok()?
}

pub async fn delete_mate(
    collection: &Collection<DBMate>,
    user_id: UserId,
    name: String,
) -> Result<DeleteResult> {
    collection
        .delete_one(doc! { "user_id": user_id.get() as i64, "name": name }, None)
        .await
        .context("Database failed to delete user; try again later!")
}

pub async fn get_all_mates(
    collection: &Collection<DBMate>,
    user_id: UserId,
) -> Result<Vec<DBMate>> {
    collection
        .find(doc! { "user_id": user_id.get() as i64 }, None)
        .await
        .context("Failed to get all mates!")?
        .try_collect::<Vec<DBMate>>()
        .await
        .context("Failed to get all mates!")
}

pub fn get_matching_mate<'a>(
    mates: &'a Vec<DBMate>,
    message_content: &String,
) -> Option<&'a DBMate> {
    for mate in mates {
        // account for proxy-tag-less mates
        if mate.prefix.is_some() || mate.postfix.is_some() {
            if message_content.starts_with(&mate.prefix.clone().unwrap_or_default())
                && message_content.ends_with(&mate.postfix.clone().unwrap_or_default())
            {
                return Some(mate);
            }
        }
    }
    None
}

pub async fn get_autoproxied_mate<'a>(
    settings_collection: &Collection<DBUserSettings>,
    mates: &'a Vec<DBMate>,
    user_id: UserId,
    guild_id: GuildId,
) -> Option<&'a DBMate> {
    let Ok(user_settings) =
        get_or_create_user_settings(settings_collection, user_id, Some(guild_id.get() as i64)).await
            else { return None };

    match user_settings.autoproxy {
        Some(AutoproxySettings::Disabled) => None,
        Some(AutoproxySettings::SwitchedIn) => {
            for mate in mates {
                if mate.autoproxy {
                    return Some(mate);
                }
            }
            None
        }
        Some(AutoproxySettings::Latch(latch)) => {
            let mate_name = match latch {
                Latch::Guild(Some(guild)) => {
                    // not sure if this is actually needed
                    if user_settings.guild_id == Some(guild_id.get() as i64) {
                        Some(guild)
                    } else {
                        None
                    }
                }
                Latch::Global(Some(global)) => Some(global),
                _ => None,
            }?;

            Some(mates.iter().filter(|mate| mate.name == mate_name).next()?)
        }
        Some(AutoproxySettings::Mate(mate_name)) => {
            Some(mates.iter().filter(|mate| mate.name == mate_name).next()?)
        }
        None => None,
    }
}
