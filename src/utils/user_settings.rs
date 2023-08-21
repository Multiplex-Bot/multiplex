use anyhow::{Context, Result};
use mongodb::{
    bson::{self, doc},
    Collection,
};
use poise::{
    futures_util::TryStreamExt,
    serenity_prelude::{Message, UserId},
};

use crate::models::{AutoproxySettings, DBUserSettings, Latch};

pub async fn get_or_create_user_settings(
    collection: &Collection<DBUserSettings>,
    user_id: UserId,
    guild_id: Option<i64>,
) -> Result<DBUserSettings> {
    fn fallback(settings: &mut DBUserSettings, fallback_settings: DBUserSettings) {
        if settings.autoproxy.is_none() {
            settings.autoproxy = fallback_settings.autoproxy;
        }
    }

    let settings = collection
        .find_one(
            doc! { "user_id": user_id.get() as i64, "guild_id": guild_id },
            None,
        )
        .await?;

    if let Some(mut settings) = settings {
        if settings.guild_id.is_some() {
            let user_settings = collection
                .find_one(
                    doc! { "user_id": user_id.get() as i64, "guild_id": None::<i64> },
                    None,
                )
                .await;

            if let Ok(Some(user_settings)) = user_settings {
                fallback(&mut settings, user_settings);
            }
        }

        Ok(settings)
    } else {
        let mut new_settings = DBUserSettings {
            user_id: user_id.get(),
            autoproxy: if guild_id.is_some() {
                None
            } else {
                Some(AutoproxySettings::SwitchedIn)
            },
            guild_id: guild_id,
            regex_sed_editing: if guild_id.is_some() { None } else { Some(true) },
        };

        collection
            .insert_one(&new_settings, None)
            .await
            .context("Failed to create new user settings in database; try again later!")?;

        if guild_id.is_some() {
            let user_settings = collection
                .find_one(
                    doc! { "user_id": user_id.get() as i64, "guild_id": None::<i64> },
                    None,
                )
                .await;

            if let Ok(Some(user_settings)) = user_settings {
                fallback(&mut new_settings, user_settings);
            } else {
                let new_user_settings = DBUserSettings {
                    user_id: user_id.get(),
                    autoproxy: Some(AutoproxySettings::SwitchedIn),
                    guild_id: None,
                    regex_sed_editing: Some(true),
                };

                collection
                    .insert_one(&new_user_settings, None)
                    .await
                    .context("Failed to create new user settings in database; try again later!")?;

                fallback(&mut new_settings, new_user_settings);
            }
        }

        Ok(new_settings)
    }
}

pub async fn update_user_settings(
    collection: &Collection<DBUserSettings>,
    settings: DBUserSettings,
    autoproxy: Option<AutoproxySettings>,
) -> Result<()> {
    let mut new_settings = settings.clone();

    if autoproxy.is_some() {
        new_settings.autoproxy = autoproxy;
    }

    collection
        .update_one(
            doc! {
                "user_id": new_settings.user_id as i64,
                "guild_id": new_settings.guild_id
            },
            doc! { "$set": bson::to_bson(&new_settings).unwrap() },
            None,
        )
        .await?;

    Ok(())
}

pub async fn update_latch(
    settings_collection: &Collection<DBUserSettings>,
    message: &Message,
    new: Option<String>,
) -> Result<()> {
    let guild_settings = get_or_create_user_settings(
        &settings_collection,
        message.author.id,
        message
            .guild_id
            .and_then(|guild_id| Some(guild_id.get() as i64)),
    )
    .await?;

    match guild_settings.autoproxy {
        Some(AutoproxySettings::Latch(Latch::Global(_))) => {
            let global_settings =
                get_or_create_user_settings(&settings_collection, message.author.id, None).await?;

            update_user_settings(
                &settings_collection,
                global_settings,
                Some(AutoproxySettings::Latch(Latch::Global(new))),
            )
            .await?;
        }
        Some(AutoproxySettings::Latch(Latch::Guild(_))) => {
            update_user_settings(
                &settings_collection,
                guild_settings,
                Some(AutoproxySettings::Latch(Latch::Guild(new))),
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}
