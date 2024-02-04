use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use mongodb::bson::doc;
use poise::{
    serenity_prelude::{
        futures::StreamExt, ComponentInteractionCollector, CreateActionRow, CreateButton,
        CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    CreateReply,
};

use super::CommandContext;
use crate::{
    models::{DBCollective, DBMate, DBMessage, DBUserSettings},
    utils::misc::envvar,
};

/// Get the statistics of the bot
#[poise::command(slash_command, ephemeral)]
pub async fn stats(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    let cache = ctx.cache();

    let user_count = mates_collection
        .distinct("user_id", None, None)
        .await?
        .len();
    let mate_count = mates_collection.count_documents(None, None).await?;
    let guild_count = cache.guild_count();

    /* ctx.say(format!(
        "Serving **{}** users (with **{}** mates) in **{}** guilds!",
        user_count, mate_count, guild_count,
    ))
    .await?; */

    let embed = CreateEmbed::new().title("Stats").fields(vec![
        ("Users", user_count.to_string(), false),
        ("Mates", mate_count.to_string(), false),
        ("Guilds", guild_count.to_string(), false),
    ]);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Ping pong 🏓
#[poise::command(slash_command, ephemeral)]
pub async fn ping(ctx: CommandContext<'_>) -> Result<()> {
    ctx.say(format!(
        "Pong :3 ({}ms)",
        (ctx.created_at().time() - Utc::now().time()).num_milliseconds()
    ))
    .await?;

    Ok(())
}

/// Join the support server!
#[poise::command(slash_command, ephemeral)]
pub async fn support(ctx: CommandContext<'_>) -> Result<()> {
    ctx.say(format!(
        "Join the support & discussion server at {}!", // fucked up
        envvar("SUPPORT_INVITE")
    ))
    .await?;

    Ok(())
}

/// Explains the purpose of the bot, and provides further links for more information
#[poise::command(slash_command)]
pub async fn explain(ctx: CommandContext<'_>) -> Result<()> {
    let embed = CreateEmbed::new().fields(vec![
        ("What is Multiplex?", "Multiplex is a \"message proxying\" bot that allows people to send messages as webhooks with custom profile pictures, names, etc.", false),
        ("Why is this used?", "Generally, these bots are used for either plural systems to identify who's talking, or roleplaying.", false),
        ("What is plurality?", "TL;DR: it's the experience of having multiple personalities in one body. (This is a very over-simplified explanation, please see https://morethanone.info for a better definition.)", false),
        ("Why are the bots talking?", "Discord shows webhooks as bots. No, they aren't real bots.", false)
    ]);

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Resets your entire collective. THIS DELETES EVERYTHING. THIS CANNOT BE UNDONE. YOU HAVE BEEN WARNED.
#[poise::command(slash_command, ephemeral)]
pub async fn reset(ctx: CommandContext<'_>) -> Result<()> {
    let reply = CreateReply::default()
        .content(
            "Are you sure you want to do this? Are you ***sure*** you want to ***delete \
             everything***? If not, please do not press yes and ignore this command.",
        )
        .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
            format!("{}reset", ctx.id()),
        )
        .label("Yes")])]);

    ctx.send(reply).await?;

    let ctx_id = ctx.id();
    let mut collector = ComponentInteractionCollector::new(&ctx.serenity_context().shard)
        .timeout(Duration::from_secs(60))
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .stream();

    while let Some(press) = collector.next().await {
        if press.data.custom_id == format!("{}reset", ctx.id()) {
            let database = &ctx.data().database;
            let mates_collection = database.collection::<DBMate>("mates");
            let collectives_collection = database.collection::<DBCollective>("collectives");
            let settings_collection = database.collection::<DBUserSettings>("settings");
            let messages_collection = database.collection::<DBMessage>("messages");

            mates_collection
                .delete_many(
                    doc! {
                        "user_id": ctx.author().id.get() as i64
                    },
                    None,
                )
                .await?;

            collectives_collection
                .delete_many(
                    doc! {
                        "user_id": ctx.author().id.get() as i64
                    },
                    None,
                )
                .await?;

            settings_collection
                .delete_many(
                    doc! {
                        "user_id": ctx.author().id.get() as i64
                    },
                    None,
                )
                .await?;

            messages_collection
                .delete_many(
                    doc! {
                        "user_id": ctx.author().id.get() as i64
                    },
                    None,
                )
                .await?;

            press
                .create_response(
                    &ctx.http(),
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .content("Your collective has been completely deleted.")
                            .components(vec![]),
                    ),
                )
                .await?;

            break;
        }
    }

    Ok(())
}
