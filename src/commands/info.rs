use std::num::NonZeroU64;

use anyhow::{bail, Context, Result};
use mongodb::bson::doc;
use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, UserId},
    CreateReply,
};

use super::CommandContext;
use crate::{
    models::{DBCollective, DBMate},
    utils,
};

// FIXME: make this less bad
/// Get the info of a user's collective or one of their mates
#[poise::command(slash_command, ephemeral)]
pub async fn info(
    ctx: CommandContext<'_>,
    #[description = "the name of the user you want to get information from (defaults to you if unspecified)"]
    user: Option<serenity::User>,
    #[description = "the name of the mate you want to get information about (if unspecified, gets collective information)"]
    mate: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    let user_id;
    if user.is_none() {
        user_id = ctx.author().id.get() as i64;
    } else {
        user_id = user.clone().unwrap().id.get() as i64;
    }
    let user_id = UserId(NonZeroU64::new(user_id as u64).unwrap());

    if let Some(mate) = mate {
        let mate = utils::get_mate(&mates_collection, user_id, mate.clone())
            .await
            .context("")?;

        if !mate.is_public && ctx.author().id != user_id {
            bail!("That mate doesn't exist!");
        }

        let mut final_embed = CreateEmbed::new()
            .title(mate.display_name.unwrap_or(mate.name))
            .thumbnail(mate.avatar);
        if let Some(bio) = mate.bio {
            final_embed = final_embed.field("Bio", bio, false);
        }
        if let Some(pronouns) = mate.pronouns {
            final_embed = final_embed.field("Pronouns", pronouns, false);
        }
        if mate.prefix.is_some() || mate.postfix.is_some() {
            final_embed = final_embed.field(
                "Selector",
                format!(
                    "{}text{}",
                    mate.prefix.unwrap_or_default(),
                    mate.postfix.unwrap_or_default()
                ),
                false,
            );
        }

        ctx.send(CreateReply::new().embed(final_embed)).await?;
    } else {
        let collectives_collection = database.collection::<DBCollective>("collectives");

        let collective = utils::get_or_create_collective(&collectives_collection, user_id).await?;

        let mates = utils::get_all_mates(&mates_collection, user_id).await?;

        let mut final_embed = CreateEmbed::new().title(
            collective
                .name
                .unwrap_or(format!("{}'s Collective", ctx.author().name)),
        );

        if let Some(bio) = collective.bio {
            final_embed = final_embed.field("Bio", bio, false);
        }

        if let Some(pronouns) = collective.pronouns {
            final_embed = final_embed.field("Pronouns", pronouns, false);
        }

        let mut mates_content = "".to_string();
        for mate in mates.iter() {
            if let Some(display_name) = mate.display_name.clone() {
                mates_content = format!(
                    "{}\n{} *({})*",
                    mates_content,
                    display_name.replace('*', "\\*"),
                    mate.name.replace('*', "\\*")
                )
            } else {
                mates_content = format!("{}\n{}", mates_content, mate.name.replace('*', "\\*"))
            }
        }

        mates_content = mates_content.trim().to_string();

        if !mates_content.is_empty() {
            final_embed = final_embed.field("Mates", mates_content, false);
        }

        ctx.send(CreateReply::new().embed(final_embed)).await?;
    }

    Ok(())
}
