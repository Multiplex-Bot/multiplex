use std::num::NonZeroU64;

use anyhow::{bail, Context, Result};
use mongodb::{bson::doc, options::FindOneOptions, results::DeleteResult, Collection, Database};
use poise::{
    futures_util::TryStreamExt,
    serenity_prelude::{
        Attachment, ChannelId, ChannelType, CreateAttachment, CreateEmbed, CreateEmbedAuthor,
        CreateWebhook, ExecuteWebhook, GuildChannel, Http, Message, MessageId, UserId, Webhook,
        WebhookId,
    },
};
use unicode_segmentation::UnicodeSegmentation;

use crate::models::{DBChannel, DBCollective, DBCollective__new, DBMate, DBMessage};

pub async fn get_mate(
    collection: &Collection<DBMate>,
    user_id: UserId,
    name: String,
) -> Option<DBMate> {
    collection
        .find_one(doc! { "user_id": user_id.get() as i64, "name": name }, None)
        .await
        .ok()?
}

pub async fn delete_mate(
    collection: &Collection<DBMate>,
    user_id: UserId,
    name: String,
) -> Result<DeleteResult> {
    collection
        .delete_one(doc! { "user_id": user_id.get() as i64, "name": name }, None)
        .await
        .context("Database failed to delete user; try again later!")
}

pub async fn get_all_mates(
    collection: &Collection<DBMate>,
    user_id: UserId,
) -> Result<Vec<DBMate>> {
    collection
        .find(doc! { "user_id": user_id.get() as i64 }, None)
        .await
        .context("Failed to get all mates!")?
        .try_collect::<Vec<DBMate>>()
        .await
        .context("Failed to get all mates!")
}

pub async fn get_or_create_collective(
    collection: &Collection<DBCollective>,
    user_id: UserId,
) -> Result<DBCollective> {
    let collective = collection
        .find_one(doc! { "user_id": user_id.get() as i64 }, None)
        .await?;

    if let Some(collective) = collective {
        Ok(collective)
    } else {
        let new_collective = DBCollective__new! {
            user_id = user_id.get() as i64,
            is_public = true,
        };

        collection
            .insert_one(new_collective.clone(), None)
            .await
            .context("Failed to create new collective in database; try again later!")?;

        Ok(new_collective)
    }
}

// FIXME: returning of a tuple for this is... a bit weird here
pub async fn get_webhook_or_create(
    http: &Http,
    collection: &Collection<DBChannel>,
    channel_id: ChannelId,
) -> Result<(Webhook, Option<ChannelId>)> {
    let channel =
        http.get_channel(channel_id).await?.guild().expect(
            "Failed to get guild channel; are you somehow sending this in a non-text channel?",
        );

    let channel_id;
    let thread_id;

    if is_thread(&channel) {
        channel_id = channel.parent_id.unwrap();
        thread_id = Some(channel.id);
    } else {
        channel_id = channel.id;
        thread_id = None;
    }

    let dbchannel = collection
        .find_one(doc! {"id": channel_id.get() as i64}, None)
        .await;

    let webhook;

    if let Ok(Some(dbchannel)) = dbchannel {
        webhook = Webhook::from_id_with_token(
            http,
            WebhookId(NonZeroU64::new(dbchannel.webhook_id as u64).unwrap()),
            &dbchannel.webhook_token,
        )
        .await?;
    } else {
        webhook = channel
            .create_webhook(http, CreateWebhook::new("Multiplex Proxier"))
            .await
            .context("Failed to create webhook")?;

        let channel = DBChannel {
            id: channel_id.get() as i64,
            webhook_id: webhook.id.get() as i64,
            webhook_token: webhook.token.clone().unwrap(),
        };

        collection
            .insert_one(channel.clone(), None)
            .await
            .context("Failed to write channel webhook to DB")?;
    }

    Ok((webhook, thread_id))
}

pub fn message_link_to_id(message_link: String) -> Result<MessageId> {
    let iter = message_link.split("/");
    let message_id = iter.last().context("Failed to get message ID from link!")?;

    Ok(MessageId(
        NonZeroU64::new(message_id.parse::<u64>()?).unwrap(),
    ))
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
    user_id: UserId,
    message_id: MessageId,
) -> Result<DBMessage> {
    let dbmessage = collection
        .find_one(doc! { "message_id": message_id.get() as i64 }, None)
        .await?
        .context("Could not find message; was it proxied by Multiplex?")?;

    if dbmessage.user_id != user_id.get() {
        bail!("This message was not sent by you or your mates!")
    } else {
        Ok(dbmessage)
    }
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

pub async fn upload_avatar(http: &Http, attachment: Attachment) -> Result<String> {
    let new_message = http
        .send_message(
            std::env::var("AVATAR_CHANNEL")
                .unwrap()
                .parse::<u64>()?
                .into(),
            vec![CreateAttachment::bytes(
                &*attachment.download().await?,
                attachment.filename.as_str(),
            )],
            &serde_json::Map::new(),
        )
        .await?;
    Ok(new_message.attachments[0].url.clone())
}

pub fn is_thread(channel: &GuildChannel) -> bool {
    channel.kind == ChannelType::PublicThread || channel.kind == ChannelType::PrivateThread
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

    let (webhook, thread_id) =
        get_webhook_or_create(http, &channels_collection, message.channel_id).await?;

    let new_content = message.content.clone();

    let mut builder = ExecuteWebhook::new();

    builder = builder
        .content(
            message
                .content
                .clone()
                .strip_prefix(&mate.prefix.clone().unwrap_or_default())
                .unwrap_or(&new_content)
                .strip_suffix(&mate.postfix.clone().unwrap_or_default())
                .unwrap_or(&new_content)
                .to_string(),
        )
        .avatar_url(mate.avatar)
        .username(format!(
            "{} {}",
            if let Some(display_name) = mate.display_name {
                display_name
            } else {
                mate.name
            },
            collective.collective_tag.unwrap_or_default()
        ));

    if let Some(referenced_message) = &message.referenced_message {
        let mut embed = CreateEmbed::new();
        let replied_graphemes = referenced_message
            .content
            .graphemes(true)
            .collect::<Vec<&str>>();

        let replied_content;
        if replied_graphemes.len() > 100 {
            replied_content = replied_graphemes[..100].join("")
        } else {
            replied_content = replied_graphemes.join("")
        }

        embed = embed.description(format!(
            "{} ([jump to message]({}))",
            replied_content,
            referenced_message.link()
        ));

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
                .unwrap_or(std::env::var("DEFAULT_AVATAR_URL")?),
        );
        embed = embed.author(author);

        builder = builder.embed(embed);
    }

    for attachment in message.attachments.iter() {
        if attachment.size >= 25000000 {
            continue;
        }
        let download = attachment.download().await?;
        builder = builder.add_file(CreateAttachment::bytes(
            std::borrow::Cow::Owned(download),
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
            },
            None,
        )
        .await?;

    Ok(())
}
