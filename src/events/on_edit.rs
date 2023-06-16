use anyhow::Result;
use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, MessageUpdateEvent};

use crate::{
    commands::Data,
    models::{DBCollective, DBMate},
    utils,
};

pub async fn run(ctx: &SerenityContext, data: &Data, message: &MessageUpdateEvent) -> Result<()> {
    let database = &data.database;

    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");

    let message = ctx
        .http()
        .get_message(message.channel_id, message.id)
        .await?;

    let mates = utils::get_all_mates(&mates_collection, message.author.id).await?;

    let mate = utils::get_matching_mate(&mates, &message.content)
        .or_else(|| utils::get_autoproxied_mate(&mates));

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
