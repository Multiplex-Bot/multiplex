use super::CommandContext;
use crate::{
    commands::UPSERT_OPTIONS,
    models::{DBCollective, DBMate},
};
use anyhow::Result;
use mongodb::bson::{self, doc};
use poise::serenity_prelude as serenity;

#[poise::command(slash_command, subcommands("mate", "collective"))]
pub async fn edit(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// Edit a mate
#[poise::command(slash_command)]
pub async fn mate(
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
            prefix: if let Some(_selector) = selector.clone() {
                prefix
            } else {
                old_mate.prefix
            },
            postfix: if let Some(_selector) = selector {
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

/// Edit details about your collective (shown on /info)
#[poise::command(slash_command)]
pub async fn collective(
    ctx: CommandContext<'_>,
    #[description = "the name of your collective"] name: Option<String>,
    #[description = "the bio of your collective"] bio: Option<String>,
    #[description = "whether your collective should be viewable by others with /info"]
    publicity: Option<bool>,
    #[description = "the collective pronouns of your collective"] pronouns: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");

    let old_collective = collectives_collection
        .find_one(doc! { "user_id": ctx.author().id.0 as i64 }, None)
        .await;

    let collective;

    if let Ok(Some(old_collective)) = old_collective {
        collective = DBCollective {
            user_id: old_collective.user_id,
            name: if let Some(name) = name {
                Some(name)
            } else {
                old_collective.name
            },
            bio: if let Some(bio) = bio {
                Some(bio)
            } else {
                old_collective.bio
            },
            pronouns: if let Some(pronouns) = pronouns {
                Some(pronouns)
            } else {
                old_collective.pronouns
            },
            is_public: if let Some(publicity) = publicity {
                publicity
            } else {
                old_collective.is_public
            },
        };
    } else {
        collective = DBCollective {
            user_id: ctx.author().id.0 as i64,
            name,
            bio,
            pronouns,
            is_public: true,
        }
    }

    collectives_collection
        .find_one_and_update(
            doc! { "user_id": ctx.author().id.0 as i64 },
            doc! { "$set": bson::to_bson(&collective).unwrap() },
            UPSERT_OPTIONS.clone().unwrap(),
        )
        .await?;

    ctx.say("Successfully updated your collective!").await?;

    Ok(())
}
