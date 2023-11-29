use std::time::Duration;

use anyhow::{bail, Context, Result};
use mongodb::bson::doc;
use poise::{
    serenity_prelude::{
        self as serenity, collector::ComponentInteractionCollector, futures::stream::StreamExt,
        CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, UserId,
    },
    CreateReply,
};

use super::CommandContext;
use crate::{
    models::{DBCollective, DBMate},
    utils::{
        collectives::get_or_create_collective,
        mates::{get_all_mates, get_mate},
    },
};

/// Get the info of a user's collective or one of their mates
#[poise::command(slash_command, ephemeral)]
pub async fn info(
    ctx: CommandContext<'_>,
    #[description = "the name of the user you want to get information from (defaults to you if \
                     unspecified)"]
    user: Option<serenity::User>,
    #[description = "the name of the mate you want to get information about (if unspecified, gets \
                     collective information)"]
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
    let user_id = UserId::new(user_id as u64);

    if let Some(mate) = mate {
        let mate = get_mate(&mates_collection, user_id, mate.clone())
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

        ctx.send(CreateReply::default().embed(final_embed)).await?;
    } else {
        let user = ctx.http().get_user(user_id).await?;

        let collectives_collection = database.collection::<DBCollective>("collectives");
        let collective = get_or_create_collective(&collectives_collection, user_id).await?;
        let mates = get_all_mates(&mates_collection, user_id).await?;

        let ctx_id = ctx.id();
        let prev_button_id = format!("{}prev", ctx_id);
        let next_button_id = format!("{}next", ctx_id);

        let mut current_page = 0;

        let mut final_embed = CreateEmbed::new().title(collective.name.unwrap_or(format!(
            "{}'s Collective",
            user.global_name.clone().unwrap_or(user.name.clone())
        )));

        if let Some(bio) = collective.bio {
            final_embed = final_embed.field("Bio", bio, false);
        }

        if let Some(pronouns) = collective.pronouns {
            final_embed = final_embed.field("Pronouns", pronouns, false);
        }

        let mates_content = mates
            .iter()
            .map(|m| {
                if let Some(display_name) = m.display_name.clone() {
                    format!(
                        "{} *({})*",
                        display_name.replace('*', "\\*"),
                        m.name.replace('*', "\\*")
                    )
                } else {
                    m.name.replace('*', "\\*")
                }
            })
            .collect::<Vec<_>>();

        if !mates_content.is_empty() {
            let mut reply = CreateReply::default().embed(final_embed.clone().field(
                "Mates",
                mates_content[0..=4.min(mates_content.len() - 1)].join("\n"),
                false,
            ));

            if 4 < mates_content.len() - 1 {
                reply = reply.components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(&prev_button_id).label("<"),
                    CreateButton::new(&next_button_id).label(">"),
                ])]);
            }

            ctx.send(reply).await?;

            if 4 >= mates_content.len() - 1 {
                return Ok(());
            }

            let mut collector = ComponentInteractionCollector::new(&ctx.serenity_context().shard)
                .timeout(Duration::from_secs(300))
                .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
                .stream();

            while let Some(press) = collector.next().await {
                if press.data.custom_id == next_button_id {
                    current_page += 1;

                    if current_page * 4 >= mates_content.len() - 1 {
                        current_page = 0;
                    }
                } else if press.data.custom_id == prev_button_id {
                    current_page = current_page.checked_sub(1).unwrap_or(0)
                }

                press
                    .create_response(
                        ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new().embed(
                                final_embed.clone().field(
                                    "Mates",
                                    mates_content[(4 * current_page) + {
                                        if current_page == 0 {
                                            0
                                        } else {
                                            1
                                        }
                                    }
                                        ..=((4 * current_page) + {
                                            if current_page == 0 {
                                                4
                                            } else {
                                                5
                                            }
                                        })
                                        .min(mates_content.len() - 1)]
                                        .join("\n"),
                                    false,
                                ),
                            ),
                        ),
                    )
                    .await?;
            }
        } else {
            ctx.send(CreateReply::default().embed(final_embed)).await?;
        }
    }

    Ok(())
}
