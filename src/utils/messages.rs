use std::borrow::Cow;

use anyhow::{bail, Context, Result};
use mongodb::{bson::doc, options::FindOneOptions, results::DeleteResult, Collection, Database};
use poise::serenity_prelude::{
    CreateAttachment, CreateEmbed, CreateEmbedAuthor, ExecuteWebhook, Http, Message, MessageId,
    UserId,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    channels::get_webhook_or_create,
    guilds::{get_or_create_dbguild, send_server_proxy_log},
    misc::envvar,
};
use crate::models::{DBChannel, DBCollective, DBGuild, DBMate, DBMessage};

pub fn clamp_message_length(content: &String) -> String {
    let replied_graphemes = content.graphemes(true).collect::<Vec<&str>>();

    if replied_graphemes.len() > 100 {
        format!("{}...", replied_graphemes[..100].join(""))
    } else {
        replied_graphemes.join("")
    }
}

pub async fn send_proxied_message(
    http: &Http,
    message: &Message,
    mate: DBMate,
    collective: DBCollective,
    database: &Database,
) -> Result<()> {
    let channels_collection = database.collection::<DBChannel>("channels");
    let messages_collection = database.collection::<DBMessage>("messages");
    let guilds_collection = database.collection::<DBGuild>("guilds");

    let (webhook, thread_id) =
        get_webhook_or_create(http, &channels_collection, message.channel_id).await?;

    let new_content = message.content.clone();
    let new_content = message
        .content
        .clone()
        .strip_prefix(&mate.prefix.clone().unwrap_or_default())
        .unwrap_or(&new_content)
        .strip_suffix(&mate.postfix.clone().unwrap_or_default())
        .unwrap_or(&new_content)
        .to_string();

    let mut builder = ExecuteWebhook::new();

    builder = builder
        .content(new_content.clone())
        .avatar_url(mate.avatar.clone())
        .username(format!(
            "{} {}",
            if let Some(display_name) = mate.display_name.clone() {
                display_name
            } else {
                mate.name.clone()
            },
            collective.collective_tag.clone().unwrap_or_default()
        ));

    if let Some(referenced_message) = &message.referenced_message {
        let author = CreateEmbedAuthor::new(format!(
            "{} ⤵️",
            referenced_message
                .author_nick(http)
                .await
                .unwrap_or(referenced_message.author.name.clone())
        ))
        .icon_url(
            referenced_message
                .author
                .avatar_url()
                .unwrap_or(envvar("DEFAULT_AVATAR_URL")),
        );

        let embed = CreateEmbed::new()
            .description(format!(
                "{} ([jump to message]({}))",
                clamp_message_length(&referenced_message.content),
                referenced_message.link()
            ))
            .author(author);

        builder = builder.embed(embed);

        if message.mentions_user_id(referenced_message.author.id) {
            builder = builder.content(format!(
                "{} ||<@{}>||",
                new_content, referenced_message.author.id
            ))
        }
    }

    for attachment in message.attachments.iter() {
        if attachment.size >= 25000000 {
            continue;
        }
        let download = attachment.download().await?;
        builder = builder.add_file(CreateAttachment::bytes(
            Cow::Owned(download),
            attachment.filename.clone(),
        ))
    }

    if let Some(thread_id) = thread_id {
        builder = builder.in_thread(thread_id)
    }

    let new_message = webhook
        .execute(http, true, builder)
        .await?
        .context("Failed to send proxied message")?;

    message.delete(http).await?;

    messages_collection
        .insert_one(
            DBMessage {
                message_id: new_message.id.0.get(),
                user_id: message.author.id.0.get(),
                mate_name: Some(mate.name.clone()),
            },
            None,
        )
        .await?;

    let guild_config =
        get_or_create_dbguild(&guilds_collection, message.guild_id.unwrap().get() as i64).await?;

    if let Some(proxy_logs_channel_id) = guild_config.proxy_logs_channel_id {
        send_server_proxy_log(
            http,
            message,
            &new_message,
            mate,
            &channels_collection,
            proxy_logs_channel_id,
        )
        .await?;
    }

    Ok(())
}

pub fn parse_selector(selector: Option<String>) -> (Option<String>, Option<String>) {
    let real_selector: String;

    if let None = selector {
        return (None, None);
    } else {
        real_selector = selector.unwrap();
    }

    let mut prefix: Option<String> = None;
    let mut postfix: Option<String> = None;

    let selector_iter: Vec<&str> = real_selector.split("text").collect();
    if selector_iter.len() == 1 {
        if real_selector.starts_with("text") {
            postfix = Some(selector_iter[0].to_string());
        } else {
            prefix = Some(selector_iter[0].to_string());
        }
    } else {
        prefix = Some(selector_iter[0].to_string());
        postfix = Some(selector_iter[1].to_string());
    }

    (prefix, postfix)
}

pub async fn get_most_recent_message(
    collection: &Collection<DBMessage>,
    user_id: UserId,
) -> Result<DBMessage> {
    collection
        .find_one(
            doc! { "user_id": user_id.get() as i64 },
            Some(FindOneOptions::builder().sort(doc! {"_id": -1}).build()),
        )
        .await?
        .context("Failed to find most recent message; try again later!")
}

pub async fn get_message(
    collection: &Collection<DBMessage>,
    user_id: Option<UserId>,
    message_id: MessageId,
) -> Result<DBMessage> {
    let dbmessage = collection
        .find_one(doc! { "message_id": message_id.get() as i64 }, None)
        .await?
        .context("Could not find message; was it proxied by Multiplex?")?;

    if let Some(user_id) = user_id {
        if dbmessage.user_id != user_id.get() {
            bail!("This message was not sent by you or your mates!")
        }
    }
    Ok(dbmessage)
}

pub async fn delete_dbmessage(
    collection: &Collection<DBMessage>,
    message_id: MessageId,
) -> Result<DeleteResult> {
    collection
        .delete_one(doc! { "message_id": message_id.get() as i64 }, None)
        .await
        .context("Database failed to delete message; try again later!")
}
