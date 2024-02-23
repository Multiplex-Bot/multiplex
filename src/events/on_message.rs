use anyhow::{Context, Result};
use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, Message, MessageFlags};

use crate::{
    commands::Data,
    models::{DBCollective, DBMate, DBUserSettings},
    utils::{
        collectives::get_or_create_collective,
        mates::{get_all_mates, get_autoproxied_mate, get_matching_mate},
        messages::send_proxied_message,
        user_settings::update_latch,
    },
};

pub async fn run(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    // FIXME: just drop voice messages until we can be bothered to actually implement proxying them
    if message
        .flags
        .context("how the FUCK does this message not have flags")?
        .intersects(MessageFlags::IS_VOICE_MESSAGE)
    {
        return Ok(());
    }

    if message.author.bot {
        return Ok(());
    }

    let database = &data.database;

    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");
    let settings_collection = database.collection::<DBUserSettings>("settings");

    let mates = get_all_mates(&mates_collection, message.author.id).await?;

    if mates.len() == 0 {
        return Ok(());
    }

    let mut mate = get_matching_mate(&mates, &message.content);

    if message.content.starts_with("\\\\") {
        update_latch(&settings_collection, message, None).await?;

        return Ok(());
    }
    if message.content.starts_with("\\") {
        return Ok(());
    }

    if mate.is_none() {
        mate = get_autoproxied_mate(
            &settings_collection,
            &mates,
            message.author.id,
            message.guild_id.unwrap(),
        )
        .await;
    } else {
        update_latch(
            &settings_collection,
            message,
            Some(mate.unwrap().name.clone()),
        )
        .await?;
    }

    if let Some(mate) = mate {
        let collective =
            get_or_create_collective(&collectives_collection, message.author.id).await?;

        return send_proxied_message(ctx.http(), message, mate.clone(), collective, database).await;
    }

    Ok(())
}
