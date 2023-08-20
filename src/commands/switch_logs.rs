use anyhow::Result;
use mongodb::bson::doc;
use poise::{serenity_prelude::CreateEmbed, CreateReply};

use super::CommandContext;
use crate::{
    models::{DBCollective, DBMate},
    utils,
};

/// Shows the last 5 switches for your collective
#[poise::command(slash_command, ephemeral)]
pub async fn switch_logs(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;
    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    let collective =
        utils::get_or_create_collective(&collectives_collection, ctx.author().id).await?;

    if let Some(switch_logs) = collective.switch_logs {
        let mut fields = vec![];

        for log in switch_logs {
            if log.unswitch {
                let previous_mate = mates_collection
                    .find_one(doc! { "_id": log.previous_mate_id.unwrap() /* on an unswitch the previous mate will always exist */ }, None)
                    .await?
                    .unwrap(); // there's no way for the mate to not exist

                fields.push((
                    log.date.to_string(),
                    format!(
                        "Unswitched from {}",
                        previous_mate.display_name.unwrap_or(previous_mate.name)
                    ),
                    false,
                ));
            } else {
                let mate = mates_collection
                    .find_one(doc! { "_id": log.mate_id.unwrap() /* no way for `mate_id` to not exist in this case */ }, None)
                    .await?
                    .unwrap(); // there's no way for the mate to not exist

                if let Some(previous_mate_id) = log.previous_mate_id {
                    let previous_mate = mates_collection
                        .find_one(doc! { "_id": previous_mate_id }, None)
                        .await?
                        .unwrap(); // there's no way for the mate to not exist

                    fields.push((
                        log.date.to_string(),
                        format!(
                            "Switched from {} to {}",
                            previous_mate.display_name.unwrap_or(previous_mate.name),
                            mate.display_name.unwrap_or(mate.name)
                        ),
                        false,
                    ));
                } else {
                    fields.push((
                        log.date.to_string(),
                        format!("Switched to {}", mate.display_name.unwrap_or(mate.name)),
                        false,
                    ));
                }
            }
        }

        let embed = CreateEmbed::new().title("Last 5 switches").fields(fields);

        ctx.send(CreateReply::new().embed(embed)).await?;
    } else {
        ctx.say("You have never switched!").await?;
    }

    Ok(())
}
