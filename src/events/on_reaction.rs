use anyhow::Result;
use poise::serenity_prelude::{
    CacheHttp, Context as SerenityContext, CreateEmbed, CreateMessage, Reaction,
};

use crate::{
    commands::Data,
    models::{DBChannel, DBMessage},
    utils::{self, get_webhook_or_create},
};

pub async fn run(ctx: &SerenityContext, data: &Data, reaction: &Reaction) -> Result<()> {
    let database = &data.database;
    let messages_collection = database.collection::<DBMessage>("messages");
    let channels_collection = database.collection::<DBChannel>("channels");

    let original_message =
        utils::get_message(&messages_collection, None, reaction.message_id).await?;

    let (webhook, thread_id) =
        get_webhook_or_create(ctx.http(), &channels_collection, reaction.channel_id).await?;

    if reaction.emoji.unicode_eq("❌") {
        if original_message.user_id == reaction.user_id.unwrap().0.get() {
            webhook
                .delete_message(ctx.http(), thread_id, reaction.message_id)
                .await?;
        }
    } else if reaction.emoji.unicode_eq("❓") {
        let real_message = ctx
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
                        .field("Mate", "Not currently implemented!", false) // FIXME: implement
                        .field(
                            "Message",
                            format!(
                                "{} ([jump to message]({}))",
                                utils::clamp_message_length(&real_message.content),
                                real_message.link()
                            ),
                            false,
                        )
                        .field(
                            "Timestamp",
                            format!("<t:{}>", real_message.timestamp.timestamp()),
                            false,
                        )]),
            )
            .await?;
        reaction.delete(ctx.http()).await?;
    }

    Ok(())
}
