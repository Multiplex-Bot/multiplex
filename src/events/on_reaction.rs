use anyhow::{Context, Result};
use poise::serenity_prelude::{
    CacheHttp, Context as SerenityContext, CreateEmbed, CreateMessage, Reaction, UserId,
};

use crate::{
    commands::Data,
    models::{DBChannel, DBMate, DBMessage},
    utils::{
        channels::get_webhook_or_create,
        mates::get_mate,
        messages::{clamp_message_length, delete_dbmessage, get_message},
    },
};

pub async fn run(ctx: &SerenityContext, data: &Data, reaction: &Reaction) -> Result<()> {
    let database = &data.database;
    let messages_collection = database.collection::<DBMessage>("messages");
    let channels_collection = database.collection::<DBChannel>("channels");

    let original_message = get_message(&messages_collection, None, reaction.message_id).await?;

    let (webhook, thread_id) =
        get_webhook_or_create(ctx.http(), &channels_collection, reaction.channel_id).await?;

    if reaction.emoji.unicode_eq("❌") {
        if original_message.user_id == reaction.user_id.unwrap().get() {
            webhook
                .delete_message(ctx.http(), thread_id, reaction.message_id)
                .await?;

            delete_dbmessage(&messages_collection, reaction.message_id).await?;
        }
    } else if reaction.emoji.unicode_eq("❓") {
        let webhook_message = ctx
            .http()
            .get_message(reaction.channel_id, reaction.message_id)
            .await?;

        reaction
            .user(ctx.http())
            .await?
            .direct_message(
                ctx.http(),
                CreateMessage::new()
                    //.content(format!("Message sent by <@{}>", original_message.user_id)),
                    .embeds(vec![CreateEmbed::new()
                        .title("Message Info")
                        .field("User", format!("<@{}>", original_message.user_id), false)
                        .field(
                            "Mate",
                            if let Some(mate_name) = original_message.mate_name {
                                let mates_collection = database.collection::<DBMate>("mates");

                                let mate = get_mate(
                                    &mates_collection,
                                    UserId::new(original_message.user_id),
                                    mate_name,
                                )
                                .await
                                .context("Failed to get mate!")?;

                                if let Some(display_name) = mate.display_name {
                                    format!("{} ({})", display_name, mate.name)
                                } else {
                                    format!("{}", mate.name)
                                }
                            } else {
                                "Unknown".to_string()
                            },
                            false,
                        )
                        .field(
                            "Message",
                            format!(
                                "{} ([jump to message]({}))",
                                clamp_message_length(&webhook_message.content),
                                webhook_message.link()
                            ),
                            false,
                        )
                        .field(
                            "Timestamp",
                            format!("<t:{}>", webhook_message.timestamp.timestamp()),
                            false,
                        )]),
            )
            .await?;
        reaction.delete(ctx.http()).await?;
    }

    Ok(())
}
