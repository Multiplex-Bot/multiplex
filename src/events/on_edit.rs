use anyhow::Result;
use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, MessageUpdateEvent};

use crate::{
    commands::Data,
    models::{DBCollective, DBMate, DBUserSettings},
    utils,
};

pub async fn run(ctx: &SerenityContext, data: &Data, message: &MessageUpdateEvent) -> Result<()> {
    if message.author.as_ref().and_then(|author| Some(author.bot)) == Some(true) {
        return Ok(());
    }

    let database = &data.database;

    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");
    let settings_collection = database.collection::<DBUserSettings>("settings");

    let message = ctx
        .http()
        .get_message(message.channel_id, message.id)
        .await?;

    let mates = utils::get_all_mates(&mates_collection, message.author.id).await?;

    if mates.len() == 0 {
        return Ok(());
    }

    let mut mate = utils::get_matching_mate(&mates, &message.content);

    if mate.is_none() {
        mate = utils::get_autoproxied_mate(
            &settings_collection,
            &mates,
            message.author.id,
            message.guild_id.unwrap(),
        )
        .await
    }

    if let Some(mate) = mate {
        let collective =
            utils::get_or_create_collective(&collectives_collection, message.author.id).await?;

        return utils::send_proxied_message(
            ctx.http(),
            &message,
            mate.clone(),
            collective,
            database,
        )
        .await;
    }

    Ok(())
}
