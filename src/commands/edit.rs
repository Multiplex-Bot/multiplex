use std::num::NonZeroU64;

use super::autocomplete::mate as mate_autocomplete;
use super::CommandContext;
use crate::{
    commands::UPSERT_OPTIONS,
    models::{DBChannel, DBCollective, DBCollective__new, DBMate, DBMate__new, DBMessage},
};
use anyhow::{Context, Result};
use mongodb::{
    bson::{self, doc},
    options::FindOneOptions,
};
use poise::serenity_prelude::{
    self as serenity, CacheHttp, CreateAttachment, EditWebhookMessage, MessageId, Webhook,
    WebhookId,
};

#[poise::command(slash_command, subcommands("mate", "collective", "message"))]
pub async fn edit(_ctx: CommandContext<'_>) -> Result<()> {
    unreachable!()
}

/// Edit a mate
#[poise::command(slash_command)]
pub async fn mate(
    ctx: CommandContext<'_>,
    #[description = "the current name of the mate"]
    #[autocomplete = "mate_autocomplete"]
    name: String,
    #[description = "the new name of the mate"] new_name: Option<String>,
    #[description = "the new trigger for proxying (ie `[text]`)"] selector: Option<String>,
    #[description = "the new name to show in chat when proxying"] display_name: Option<String>,
    #[description = "whether to allow other people to use /info for this mate"] publicity: Option<
        bool,
    >,
    #[description = "the new avatar to use when proxying"] avatar: Option<serenity::Attachment>,
    #[description = "the mate's bio"] bio: Option<String>,
    #[description = "the mate's pronouns"] pronouns: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let mates_collection = database.collection::<DBMate>("mates");

    let old_mate = mates_collection
        .find_one(
            doc! { "user_id": ctx.author().id.0.get() as i64, "name": name.clone() },
            None,
        )
        .await;

    if let Ok(Some(old_mate)) = old_mate {
        let mut prefix: Option<String> = None;
        let mut postfix: Option<String> = None;

        if let Some(selector) = selector.clone() {
            let selector_iter: Vec<&str> = selector.split("text").collect();
            if selector_iter.len() == 1 {
                if selector.starts_with("text") {
                    postfix = Some(selector_iter[0].to_string());
                } else {
                    prefix = Some(selector_iter[0].to_string());
                }
            } else {
                prefix = Some(selector_iter[0].to_string());
                postfix = Some(selector_iter[1].to_string());
            }
        }

        let mate = DBMate__new! {
            user_id = ctx.author().id.0.get() as i64,
            name = if let Some(new_name) = new_name {
                new_name
            } else {
                name.clone()
            },
            prefix = if let Some(_selector) = selector.clone() {
                prefix
            } else {
                old_mate.prefix
            },
            postfix = if let Some(_selector) = selector {
                postfix
            } else {
                old_mate.postfix
            },
            avatar = if let Some(avatar) = avatar {
                let new_message = ctx
                    .http()
                    .send_message(
                        std::env::var("AVATAR_CHANNEL").unwrap().parse::<u64>()?.into(),
                        vec![CreateAttachment::bytes(&*avatar.download().await?, avatar.filename.as_str())],
                        &serde_json::Map::new(),
                    )
                    .await?;
                new_message.attachments[0].url.clone()
            } else {
                old_mate.avatar
            },
            bio = if let Some(bio) = bio {
                Some(bio)
            } else {
                old_mate.bio
            },
            pronouns = if let Some(pronouns) = pronouns {
                Some(pronouns)
            } else {
                old_mate.pronouns
            },
            display_name = if let Some(display_name) = display_name {
                Some(display_name)
            } else {
                old_mate.display_name
            },
            is_public = if let Some(publicity) = publicity {
                publicity
            } else {
                old_mate.is_public
            },
            autoproxy = old_mate.autoproxy,
        };

        mates_collection
            .find_one_and_replace(
                doc! { "user_id": ctx.author().id.0.get() as i64, "name": name.clone() },
                mate,
                None,
            )
            .await?;

        ctx.say("Successfully edited mate!").await?;
    } else {
        ctx.say("You can't edit a non-existent mate!").await?;
    }

    Ok(())
}

/// Edit details about your collective (shown on /info)
#[poise::command(slash_command)]
pub async fn collective(
    ctx: CommandContext<'_>,
    #[description = "the name of your collective"] name: Option<String>,
    #[description = "the bio of your collective"] bio: Option<String>,
    #[description = "whether your collective should be viewable by others with /info"]
    publicity: Option<bool>,
    #[description = "the collective pronouns of your collective"] pronouns: Option<String>,
    #[description = "A tag to append to all proxied mates, to identify your collective in chat"]
    collective_tag: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");

    let old_collective = collectives_collection
        .find_one(doc! { "user_id": ctx.author().id.0.get() as i64 }, None)
        .await;

    let collective;

    if let Ok(Some(old_collective)) = old_collective {
        collective = DBCollective__new! {
            user_id = old_collective.user_id,
            name = if let Some(name) = name {
                Some(name)
            } else {
                old_collective.name
            },
            bio = if let Some(bio) = bio {
                Some(bio)
            } else {
                old_collective.bio
            },
            pronouns = if let Some(pronouns) = pronouns {
                Some(pronouns)
            } else {
                old_collective.pronouns
            },
            is_public = if let Some(publicity) = publicity {
                publicity
            } else {
                old_collective.is_public
            },
            collective_tag = if let Some(collective_tag) = collective_tag {
                Some(collective_tag)
            } else {
                old_collective.collective_tag
            },
        };
    } else {
        collective = DBCollective__new! {
            user_id = ctx.author().id.0.get() as i64,
            name,
            bio,
            pronouns,
            collective_tag,
            is_public = true,
        }
    }

    collectives_collection
        .find_one_and_update(
            doc! { "user_id": ctx.author().id.0.get() as i64 },
            doc! { "$set": bson::to_bson(&collective).unwrap() },
            UPSERT_OPTIONS.clone().unwrap(),
        )
        .await?;

    ctx.say("Successfully updated your collective!").await?;

    Ok(())
}

/// Edit a proxied message in the current channel (if none is specified, edit the most recent one)
#[poise::command(slash_command)]
pub async fn message(
    ctx: CommandContext<'_>,
    #[description = "The new message content"] content: String,
    #[description = "The raw ID of the message to edit"] message_id: Option<String>,
    #[description = "A link to the message to edit"] message_link: Option<String>,
) -> Result<()> {
    let database = &ctx.data().database;
    let channels_collection = database.collection::<DBChannel>("channels");
    let messages_collection = database.collection::<DBMessage>("messages");

    let channel = channels_collection
        .find_one(doc! {"id": ctx.channel_id().0.get() as i64}, None)
        .await?
        .context("oopsie daisy")?;

    let message_to_edit_id;
    if let Some(message_id) = message_id {
        message_to_edit_id = MessageId(NonZeroU64::new(message_id.parse::<u64>()?).unwrap())
    } else if let Some(message_link) = message_link {
        // https://discord.com/channels/891039687785996328/1008966348862390312/1109539731655626763 -> 1109539731655626763
        let iter = message_link.split("/");
        let message_id = iter.last().context("Failed to get message ID from link!")?;
        message_to_edit_id = MessageId(NonZeroU64::new(message_id.parse::<u64>()?).unwrap())
    } else {
        let message = messages_collection
            .find_one(
                doc! { "user_id": ctx.author().id.0.get() as i64 },
                Some(FindOneOptions::builder().sort(doc! {"_id": -1}).build()),
            )
            .await?
            .context("Failed to get most recent message!")?;
        message_to_edit_id = MessageId(NonZeroU64::new(message.message_id).unwrap())
    }

    messages_collection
        .find_one(
            doc! { "message_id": i64::from(message_to_edit_id), "user_id": ctx.author().id.0.get() as i64 },
            None,
        )
        .await?
        .context("Sorry, the proxied message is not available; leave a message at the tone.")?;

    let webhook = Webhook::from_id_with_token(
        ctx.http(),
        WebhookId(NonZeroU64::new(channel.webhook_id as u64).unwrap()),
        &channel.webhook_token,
    )
    .await?;

    webhook
        .edit_message(
            ctx.http(),
            message_to_edit_id,
            EditWebhookMessage::new().content(content),
        )
        .await?;

    ctx.say("Edited message!").await?;

    Ok(())
}
