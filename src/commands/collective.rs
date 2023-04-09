use crate::models::DBCollective;

use super::{CommandContext, UPSERT_OPTIONS};
use anyhow::Result;
use mongodb::bson::{self, doc};

/// Edit details about your collective (shown on /info)
#[poise::command(slash_command)]
pub async fn edit_collective(
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
