use anyhow::Result;
use mongodb::bson::{self, doc};
use poise::serenity_prelude::{self};

use super::CommandContext;
use crate::{
    commands::UPSERT_OPTIONS,
    models::{DBCollective, DBMate},
    pluralkit::PluralkitExport,
    tupperbox::TupperboxExport,
};

#[poise::command(slash_command, subcommands("tupperbox", "pluralkit"))]
pub async fn import(_ctx: CommandContext<'_>) -> Result<()> {
    // This can't be reached without a prefix command, which we don't use
    unreachable!()
}

// FIXME: upsertion of mates should be converted to a util at some point in the future

/// Import a Tupperbox export into your collective (WARNING: WILL OVERWRITE CURRENT COLLECTIVE MEMBERS)
#[poise::command(slash_command, ephemeral)]
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
        let mate = tupper.to_mate()?;

        mates_collection
            .find_one_and_update(
                doc! { "user_id": ctx.author().id.get() as i64, "name": mate.name.clone() },
                doc! { "$set": bson::to_bson(&mate).unwrap() },
                UPSERT_OPTIONS.clone().unwrap(),
            )
            .await?;
    }

    ctx.say(
        "Successfully imported Tupperbox export! (Note: exported Tuppers may have taken \
         precedence over any existing mates. If so, sorry! They're unrecoverable!)",
    )
    .await?;

    Ok(())
}

/// Import a Multiplex export into your collective (WARNING: WILL OVERWRITE ALL COLLECTIVE INFORMATION)
#[poise::command(slash_command, ephemeral)]
pub async fn multiplex(
    ctx: CommandContext<'_>,
    #[description = "your multiplex export"] file: serenity_prelude::Attachment,
) -> Result<()> {
    let download = file.download().await?;
    let content = String::from_utf8_lossy(&download);

    let export: PluralkitExport = serde_json::from_str(&content)?;

    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    collectives_collection
        .find_one_and_update(
            doc! { "user_id": ctx.author().id.get() as i64 },
            doc! { "$set": bson::to_bson(&export.to_collective(ctx.author().id)?).unwrap() },
            UPSERT_OPTIONS.clone().unwrap(),
        )
        .await?;

    for member in export.members.iter() {
        let mate = member.to_mate(ctx.author().id)?;

        mates_collection
            .find_one_and_update(
                doc! { "user_id": ctx.author().id.get() as i64, "name": mate.name.clone() },
                doc! { "$set": bson::to_bson(&mate).unwrap() },
                UPSERT_OPTIONS.clone().unwrap(),
            )
            .await?;
    }

    ctx.say(
        "Successfully imported Multiplex export! (Note: exported Members may have taken \
         precedence over any existing mates, and your collective information was completely \
         overwritten with whatever you gave us. If so, sorry! They're unrecoverable!)",
    )
    .await?;

    Ok(())
}

/// Import a Pluralkit export into your collective (WARNING: WILL OVERWRITE ALL COLLECTIVE INFORMATION)
#[poise::command(slash_command, ephemeral)]
pub async fn pluralkit(
    ctx: CommandContext<'_>,
    #[description = "your pluralkit export"] file: serenity_prelude::Attachment,
) -> Result<()> {
    let download = file.download().await?;
    let content = String::from_utf8_lossy(&download);

    let export: PluralkitExport = serde_json::from_str(&content)?;

    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    collectives_collection
        .find_one_and_update(
            doc! { "user_id": ctx.author().id.get() as i64 },
            doc! { "$set": bson::to_bson(&export.to_collective(ctx.author().id)?).unwrap() },
            UPSERT_OPTIONS.clone().unwrap(),
        )
        .await?;

    for member in export.members.iter() {
        let mate = member.to_mate(ctx.author().id)?;

        mates_collection
            .find_one_and_update(
                doc! { "user_id": ctx.author().id.get() as i64, "name": mate.name.clone() },
                doc! { "$set": bson::to_bson(&mate).unwrap() },
                UPSERT_OPTIONS.clone().unwrap(),
            )
            .await?;
    }

    ctx.say(
        "Successfully imported Pluralkit export! (Note: exported Members may have taken \
         precedence over any existing mates, and your collective information was completely \
         overwritten with whatever PK gave us. If so, sorry! They're unrecoverable!)",
    )
    .await?;

    Ok(())
}
