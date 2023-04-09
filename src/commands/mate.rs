use std::primitive;

use super::CommandContext;
use crate::models::DBMate;
use anyhow::{Context, Error, Result};
use mongodb::bson::{self, doc};
use poise::serenity_prelude::{self as serenity, CacheHttp};

/// Register a new mate
#[poise::command(slash_command)]
pub async fn create(
    ctx: CommandContext<'_>,
    #[description = "the name of the mate"] name: String,
    #[description = "the trigger for proxying (ie `[text]`)"] selector: String,
    #[description = "whether to allow other people to use /info for this mate"] publicity: Option<
        bool,
    >,
    #[description = "the name to show in chat when proxying (otherwise use the full name)"]
    display_name: Option<String>,
    #[description = "an (optional) avatar to use when proxying"] avatar: Option<
        serenity::Attachment,
    >,
    #[description = "the mate's bio"] bio: Option<String>,
    #[description = "the mate's pronouns"] pronouns: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    let old_mate = mates_collection
        .find_one(
            doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(_)) = old_mate {
        ctx.say("You cannot have more than one mate with the same actual name!")
            .await?;
    } else {
        let mut prefix: Option<String> = None;
        let mut postfix: Option<String> = None;

        let selector_iter: Vec<&str> = selector.split("text").collect();
        if selector_iter.len() == 1 {
            if selector.starts_with("text") {
                postfix = Some(selector_iter[0].to_string());
            } else {
                prefix = Some(selector_iter[0].to_string());
            }
        } else {
            prefix = Some(selector_iter[0].to_string());
            postfix = Some(selector_iter[1].to_string());
        }

        let avatar_url;

        if let Some(avatar) = avatar {
            avatar_url = avatar.url
        } else {
            avatar_url = std::env::var("DEFAULT_AVATAR_URL").unwrap();
        }

        let mate = DBMate {
            user_id: ctx.author().id.0 as i64,
            name: name.clone(),
            is_public: publicity.unwrap_or(true),
            prefix,
            postfix,
            avatar: avatar_url,
            bio,
            pronouns,
            display_name,
            autoproxy: false,
        };

        mates_collection.insert_one(mate, None).await?;

        ctx.say(format!("Successfully created mate '{}'! :3", name))
            .await?;
    }

    Ok(())
}

/// Edit a mate
#[poise::command(slash_command)]
pub async fn edit(
    ctx: CommandContext<'_>,
    #[description = "the current name of the mate"] name: String,
    #[description = "the new name of the mate"] new_name: Option<String>,
    #[description = "the new trigger for proxying (ie `[text]`)"] selector: Option<String>,
    #[description = "the new name to show in chat when proxying"] display_name: Option<String>,
    #[description = "whether to allow other people to use /info for this mate"] publicity: Option<
        bool,
    >,
    #[description = "the new avatar to use when proxying"] avatar: Option<serenity::Attachment>,
    #[description = "the mate's bio"] bio: Option<String>,
    #[description = "the mate's pronouns"] pronouns: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    let old_mate = mates_collection
        .find_one(
            doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(old_mate)) = old_mate {
        let mut prefix: Option<String> = None;
        let mut postfix: Option<String> = None;

        if let Some(selector) = selector.clone() {
            let selector_iter: Vec<&str> = selector.split("text").collect();
            if selector_iter.len() == 1 {
                if selector.starts_with("text") {
                    postfix = Some(selector_iter[0].to_string());
                } else {
                    prefix = Some(selector_iter[0].to_string());
                }
            } else {
                prefix = Some(selector_iter[0].to_string());
                postfix = Some(selector_iter[1].to_string());
            }
        }

        let mate = DBMate {
            user_id: ctx.author().id.0 as i64,
            name: if let Some(new_name) = new_name {
                new_name
            } else {
                name.clone()
            },
            prefix: if let Some(selector) = selector.clone() {
                prefix
            } else {
                old_mate.prefix
            },
            postfix: if let Some(selector) = selector {
                postfix
            } else {
                old_mate.postfix
            },
            avatar: if let Some(avatar) = avatar {
                avatar.url
            } else {
                old_mate.avatar
            },
            bio: if let Some(bio) = bio {
                Some(bio)
            } else {
                old_mate.bio
            },
            pronouns: if let Some(pronouns) = pronouns {
                Some(pronouns)
            } else {
                old_mate.pronouns
            },
            display_name: if let Some(display_name) = display_name {
                Some(display_name)
            } else {
                old_mate.display_name
            },
            is_public: if let Some(publicity) = publicity {
                publicity
            } else {
                old_mate.is_public
            },
            autoproxy: old_mate.autoproxy,
        };

        mates_collection
            .find_one_and_replace(
                doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
                mate,
                None,
            )
            .await?;

        ctx.say("Successfully edited mate!").await?;
    } else {
        ctx.say("You can't edit a non-existent mate!").await?;
    }

    Ok(())
}

/// Set your switch (ie your default proxy)
#[poise::command(slash_command)]
pub async fn switch(
    ctx: CommandContext<'_>,
    #[description = "the name of the mate to switch to (removes current switch if not set)"]
    name: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    if let Some(name) = name {
        let mate = mates_collection
            .find_one(
                doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
                None,
            )
            .await;

        if let Ok(Some(_)) = mate {
            mates_collection
                .update_one(
                    doc! { "user_id": ctx.author().id.0 as i64, "autoproxy": true },
                    doc! { "$set": {"autoproxy": false} },
                    None,
                )
                .await?;

            mates_collection
                .update_one(
                    doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
                    doc! { "$set": {"autoproxy": true} },
                    None,
                )
                .await?;

            ctx.say(format!("Switched to {}!", name)).await?;
        } else {
            ctx.say("You need a mate with that name to switch to them!")
                .await?;
        }
    } else {
        mates_collection
            .update_one(
                doc! { "user_id": ctx.author().id.0 as i64, "autoproxy": true },
                doc! { "$set": {"autoproxy": false} },
                None,
            )
            .await?;

        ctx.say("Removed current switch!").await?;
    }

    Ok(())
}

/// Delete a mate
#[poise::command(slash_command)]
pub async fn delete(
    ctx: CommandContext<'_>,
    #[description = "the name of the mate"] name: String,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    let old_mate = mates_collection
        .find_one(
            doc! { "user_id": ctx.author().id.0 as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(_)) = old_mate {
        mates_collection
            .delete_one(
                doc! { "user_id": ctx.author().id.0 as i64,"name": name.clone() },
                None,
            )
            .await?;
        ctx.say(format!("{} has been removed. o7", name)).await?;
    } else {
        ctx.say("You need a mate with that name to remove them!")
            .await?;
    }

    Ok(())
}
