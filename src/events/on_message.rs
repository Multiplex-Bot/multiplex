use anyhow::Result;
use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, Message};

use crate::{
    commands::Data,
    models::{DBCollective, DBMate, DBUserSettings},
    utils,
};

pub async fn run(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    if message.author.bot {
        return Ok(());
    }

    let database = &data.database;

    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");
    let settings_collection = database.collection::<DBUserSettings>("settings");

    let mates = utils::get_all_mates(&mates_collection, message.author.id).await?;

    if mates.len() == 0 {
        return Ok(());
    }

    let mut mate = utils::get_matching_mate(&mates, &message.content);

    if message.content.starts_with("\\") {
        utils::update_latch(&settings_collection, message, None).await?;

        return Ok(());
    }

    if mate.is_none() {
        mate = utils::get_autoproxied_mate(
            &settings_collection,
            &mates,
            message.author.id,
            message.guild_id.unwrap(),
        )
        .await
    } else {
        utils::update_latch(
            &settings_collection,
            message,
            Some(mate.unwrap().name.clone()),
        )
        .await?;
    }

    if let Some(mate) = mate {
        let collective =
            utils::get_or_create_collective(&collectives_collection, message.author.id).await?;

        return utils::send_proxied_message(
            ctx.http(),
            message,
            mate.clone(),
            collective,
            database,
        )
        .await;
    }

    Ok(())
}
