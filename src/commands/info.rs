use super::CommandContext;
use crate::models::{DBCollective, DBMate};
use anyhow::{Context, Result};
use mongodb::bson::doc;
use poise::{
    futures_util::TryStreamExt,
    serenity_prelude::{self as serenity, CreateEmbed, Embed},
};

/// Get the info of a user's collective or one of their mates
#[poise::command(slash_command)]
pub async fn info(
    ctx: CommandContext<'_>,
    #[description = "the name of the user you want to get information from (defaults to you if unspecified)"]
    mut user: Option<serenity::User>,
    #[description = "the name of the mate you want to get information about (if unspecified, gets collective information)"]
    mate: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    let user_id;
    if let None = user {
        user = Some(ctx.author().clone());
        user_id = ctx.author().id.0 as i64;
    } else {
        user_id = user.clone().unwrap().id.0 as i64;
    }

    if let Some(mate) = mate {
        let mate = mates_collection
            .find_one(doc! { "user_id": user_id, "name": mate.clone() }, None)
            .await;

        if let Ok(Some(mate)) = mate {
            if !mate.is_public && ctx.author().id.0 as i64 != user_id {
                return Err(anyhow::anyhow!("That mate doesn't exist!"));
            }

            ctx.send(|b| {
                b.embed(|mut final_embed| {
                    final_embed = final_embed.title(mate.display_name.unwrap_or(mate.name));
                    if let Some(bio) = mate.bio {
                        final_embed = final_embed.field("Bio", bio, false);
                    }
                    if let Some(pronouns) = mate.pronouns {
                        final_embed = final_embed.field("Pronouns", pronouns, false);
                    }
                    final_embed = final_embed.field(
                        "Selector",
                        format!(
                            "{}text{}",
                            mate.prefix.unwrap_or_default(),
                            mate.postfix.unwrap_or_default()
                        ),
                        false,
                    );
                    final_embed
                })
            })
            .await?;
        } else {
            return Err(anyhow::anyhow!("That mate doesn't exist!"));
        }
    } else {
        let default_collective = DBCollective {
            user_id,
            name: Some(format!("{}'s Collective", user.unwrap().name)),
            bio: None,
            pronouns: None,
            is_public: true,
        };

        let collectives_collection = database.collection::<DBCollective>("collectives");

        let collective = collectives_collection
            .find_one(doc! { "user_id": user_id }, None)
            .await
            // NOTE: I've seen it error on both of these whenever there's not a result for the query, so I'm not sure which it actually should be
            .unwrap_or(Some(default_collective.clone()))
            .unwrap_or(default_collective);

        if !collective.is_public && ctx.author().id.0 as i64 != user_id {
            return Err(anyhow::anyhow!("That is a private collective!"));
        }

        let mates = mates_collection
            .find(doc! {"user_id": user_id }, None)
            .await
            .context("Failed to get user's mates")?;

        let mates = mates.try_collect::<Vec<DBMate>>().await?;

        ctx.send(|b| {
            b.embed(|mut e| {
                e = e.title(
                    collective
                        .name
                        .unwrap_or(format!("{}'s Collective", ctx.author().name)),
                );
                if let Some(bio) = collective.bio {
                    e = e.field("Bio", bio, false);
                }
                if let Some(pronouns) = collective.pronouns {
                    e = e.field("Pronouns", pronouns, false);
                }
                let mut mates_content = "".to_string();
                for mate in mates.iter() {
                    if let Some(display_name) = mate.display_name.clone() {
                        mates_content = format!(
                            "{}\n{} *({})*",
                            mates_content,
                            display_name.replace("*", "\\*"),
                            mate.name.replace("*", "\\*")
                        )
                    } else {
                        mates_content =
                            format!("{}\n{}", mates_content, mate.name.replace("*", "\\*"))
                    }
                }
                mates_content = mates_content.trim().to_string();
                if mates_content != "" {
                    e = e.field("Mates", mates_content, false);
                }
                e
            })
        })
        .await?;
    }

    Ok(())
}
