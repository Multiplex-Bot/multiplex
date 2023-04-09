use super::CommandContext;
use crate::{commands::UPSERT_OPTIONS, models::DBMate, tupperbox::TupperboxExport};
use anyhow::{Context, Error, Result};
use mongodb::bson::{self, doc};
use poise::serenity_prelude::{self, CacheHttp};

///
#[poise::command(slash_command, subcommands("tupperbox", "pluralkit"))]
pub async fn import(ctx: CommandContext<'_>) -> Result<()> {
    // This can't be reached without a prefix command, which we don't use
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn tupperbox(
    ctx: CommandContext<'_>,
    #[description = "your tupperbox export"] file: serenity_prelude::Attachment,
) -> Result<()> {
    let download = file.download().await?;
    let content = String::from_utf8_lossy(&download);

    let export: TupperboxExport = serde_json::from_str(&content)?;

    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    for tupper in export.tuppers.iter() {
        let mate = tupper.into_mate()?;

        mates_collection
            .find_one_and_update(
                doc! { "user_id": ctx.author().id.0 as i64, "name": mate.name.clone() },
                doc! { "$set": bson::to_bson(&mate).unwrap() },
                UPSERT_OPTIONS.clone().unwrap(),
            )
            .await?;
    }

    ctx.say("Successfully imported Tupperbox export! (Note: exported Tuppers may have taken precedence over any existing mates. If so, sorry! They're unrecoverable!)").await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn pluralkit(
    ctx: CommandContext<'_>,
    #[description = "your pluralkit export"] file: serenity_prelude::Attachment,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "Pluralkit importing is not supported at the moment!"
    ))
}
