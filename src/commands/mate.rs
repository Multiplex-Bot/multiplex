use anyhow::{bail, Result};
use mongodb::bson::doc;
use poise::serenity_prelude::{self as serenity};

use super::{autocomplete::mate as mate_autocomplete, CommandContext};
use crate::{
    models::{DBCollective, DBMate, DBMate__new},
    utils::{
        collectives::{get_or_create_collective, update_switch_logs},
        messages::parse_selector,
        misc::{envvar, upload_avatar},
    },
};

/// Register a new mate
#[poise::command(slash_command, ephemeral)]
pub async fn create(
    ctx: CommandContext<'_>,
    #[description = "the name of the mate"] name: String,
    #[description = "the trigger for proxying (ie `[text]`)"] selector: Option<String>,
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
            doc! { "user_id": ctx.author().id.get() as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(_)) = old_mate {
        ctx.say("You cannot have more than one mate with the same actual name!")
            .await?;
    } else {
        let (prefix, postfix) = parse_selector(selector);

        let avatar_url;

        if let Some(avatar) = avatar {
            avatar_url = upload_avatar(
                &ctx.data().avatar_bucket,
                ctx.author().id,
                name.clone(),
                avatar,
            )
            .await?;
        } else {
            avatar_url = envvar("DEFAULT_AVATAR_URL");
        }

        let mate = DBMate__new! {
            user_id = ctx.author().id.get() as i64,
            name = name.clone(),
            is_public = publicity.unwrap_or(true),
            prefix,
            postfix,
            avatar = avatar_url,
            bio,
            pronouns,
            display_name,
            autoproxy = false,
        };

        mates_collection.insert_one(mate, None).await?;

        ctx.say(format!("Successfully created mate '{}'! :3", name))
            .await?;
    }

    Ok(())
}

/// Set your switch (ie your default proxy)
#[poise::command(slash_command, ephemeral)]
pub async fn switch(
    ctx: CommandContext<'_>,
    #[description = "the name of the mate to switch to (removes current switch if not set)"]
    #[autocomplete = "mate_autocomplete"]
    name: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");

    let previous_mate = mates_collection
        .find_one(
            doc! {"user_id": ctx.author().id.get() as i64, "autoproxy": true},
            None,
        )
        .await?;

    let collective = get_or_create_collective(&collectives_collection, ctx.author().id).await?;

    if let Some(name) = name {
        let mate = mates_collection
            .find_one(
                doc! { "user_id": ctx.author().id.get() as i64, "name": name.clone() },
                None,
            )
            .await?;

        if let Some(mate) = mate {
            mates_collection
                .update_one(
                    doc! { "user_id": mate.user_id, "autoproxy": true },
                    doc! { "$set": {"autoproxy": false} },
                    None,
                )
                .await?;

            mates_collection
                .update_one(
                    doc! { "user_id": mate.user_id, "name": &name },
                    doc! { "$set": {"autoproxy": true} },
                    None,
                )
                .await?;

            update_switch_logs(
                &collectives_collection,
                &collective,
                Some(mate.id.unwrap()),
                previous_mate.and_then(|m| Some(m.id.unwrap())),
            )
            .await?;

            ctx.say(format!("Switched to {}!", name)).await?;
        } else {
            bail!("You need a mate with that name to switch to them!")
        }
    } else {
        if previous_mate.is_none() {
            ctx.say("No mate is currently switched in!").await?;

            return Ok(());
        }

        mates_collection
            .update_one(
                doc! { "user_id": ctx.author().id.get() as i64, "autoproxy": true },
                doc! { "$set": {"autoproxy": false} },
                None,
            )
            .await?;

        update_switch_logs(
            &collectives_collection,
            &collective,
            None,
            previous_mate.and_then(|m| Some(m.id.unwrap())),
        )
        .await?;

        ctx.say("Removed current switch!").await?;
    }

    Ok(())
}
