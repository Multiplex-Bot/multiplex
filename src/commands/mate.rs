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
    #[description = "an (optional) avatar to use when proxying"] avatar: Option<
        serenity::Attachment,
    >,
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
            prefix,
            postfix,
            avatar: avatar_url,
            ..Default::default()
        };

        mates_collection.insert_one(mate, None).await?;

        ctx.say(format!("Successfully created mate '{}'! :3", name))
            .await?;
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
