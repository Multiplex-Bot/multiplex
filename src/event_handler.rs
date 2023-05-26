use anyhow::{Context, Result};
use mongodb::bson::doc;
use mongodb::Database;
use poise::futures_util::TryStreamExt;

use poise::serenity_prelude::{
    webhook, AttachmentType, CacheHttp, Channel, ChannelType, Context as SerenityContext, Message,
    MessageUpdateEvent, Webhook, WebhookId,
};

use crate::commands::Data;
use crate::models::{DBChannel, DBCollective, DBCollective__new, DBMate, DBMessage};

async fn send_proxied_message(
    ctx: &SerenityContext,
    message: &Message,
    channel: DBChannel,
    mate: DBMate,
    collective: DBCollective,
    database: &Database,
) -> Result<()> {
    let messages_collection = database.collection::<DBMessage>("messages");

    let webhook = Webhook::from_id_with_token(
        ctx.http(),
        WebhookId(channel.webhook_id as u64),
        &channel.webhook_token,
    )
    .await?;

    let mut new_content = message.content.clone();

    new_content = new_content
        .strip_prefix(&mate.prefix.clone().unwrap_or_default())
        .unwrap_or(&new_content)
        .strip_suffix(&mate.postfix.clone().unwrap_or_default())
        .unwrap_or(&new_content)
        .to_string();

    if let Some(referenced_message) = &message.referenced_message {
        // FIXME: this whole reply system is *really* jank, and will break if anyone uses a zero-width space in a message.
        new_content = format!(
            "> {}\n[jump to content](https://discord.com/channels/{}/{}/{}) {}\n\u{200B}{}\u{200B}",
            if referenced_message.webhook_id.is_some() {
                let mut message_parts = referenced_message
                    .content
                    .split('\u{200B}')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                message_parts.reverse();

                message_parts
                    .get(1)
                    .unwrap_or(&referenced_message.content)
                    .replace('\n', "\n> ")
            } else {
                referenced_message.content.replace('\n', "\n> ")
            },
            message.guild_id.unwrap().0,
            referenced_message.channel_id.0,
            referenced_message.id.0,
            if message.mentions_user(&referenced_message.author) {
                if referenced_message.webhook_id.is_some() {
                    format!(
                        "- {}",
                        referenced_message
                            .author_nick(ctx.http())
                            .await
                            .unwrap_or(referenced_message.author.name.clone())
                    )
                } else {
                    format!("- <@{}>", referenced_message.author.id.0)
                }
            } else {
                format!(
                    "- {}",
                    referenced_message
                        .author_nick(ctx.http())
                        .await
                        .unwrap_or(referenced_message.author.name.clone())
                )
            },
            new_content
        );
    }

    let mut reattachments: Vec<AttachmentType> = vec![];

    for attachment in message.attachments.iter() {
        if attachment.size >= 25000000 {
            return Ok(());
        }
        let download = attachment.download().await?;
        reattachments.push(AttachmentType::Bytes {
            data: std::borrow::Cow::Owned(download),
            filename: attachment.filename.clone(),
        });
    }

    let new_message = webhook
        .execute(ctx.http(), true, |msg| {
            msg.avatar_url(mate.avatar)
                .username(format!(
                    "{} {}",
                    if let Some(display_name) = mate.display_name {
                        display_name
                    } else {
                        mate.name
                    },
                    collective.collective_tag.unwrap_or_default()
                ))
                .content(new_content)
                .add_files(reattachments)
        })
        .await?
        .context("Failed to send webhook message (I think)")?;
    message.delete(ctx.http()).await?;

    messages_collection
        .insert_one(
            DBMessage {
                message_id: new_message.id.0,
                user_id: message.author.id.0,
            },
            None,
        )
        .await?;

    Ok(())
}

pub async fn on_edit(
    ctx: &SerenityContext,
    data: &Data,
    message: &MessageUpdateEvent,
) -> Result<()> {
    let database = &data.database;

    let channels_collection = database.collection::<DBChannel>("channels");
    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");

    let dbchannel = channels_collection
        .find_one(doc! {"id": message.channel_id.0 as i64}, None)
        .await;

    let channel;

    if let Ok(Some(dbchannel)) = dbchannel {
        channel = dbchannel;
    } else {
        let guild_channel = ctx
            .http()
            .get_channel(message.channel_id.0)
            .await?
            .guild()
            .context("Failed to get guild channel")?;

        let webhook = guild_channel
            .create_webhook(ctx.http(), "Multiplex Proxier")
            .await
            .context("Failed to create webhook")?;

        channel = DBChannel {
            id: guild_channel.id.0 as i64,
            webhook_id: webhook.id.0 as i64,
            webhook_token: webhook.token.unwrap(),
        };

        channels_collection
            .insert_one(channel.clone(), None)
            .await
            .context("Failed to write channel webhook to DB")?;
    }

    let mates = mates_collection
        .find(
            doc! {"user_id": message.author.clone().unwrap().id.0 as i64 },
            None,
        )
        .await
        .context("Failed to get user's mates")?;

    let mates = mates.try_collect::<Vec<DBMate>>().await?;

    let default_collective = DBCollective__new! {
        user_id = message.author.clone().unwrap().id.0 as i64,
        is_public = true,
    };

    let collective = collectives_collection
        .find_one(
            doc! { "user_id": message.author.clone().unwrap().id.0 as i64 },
            None,
        )
        .await
        .unwrap_or(Some(default_collective.clone()))
        .unwrap_or(default_collective);

    for mate in mates.clone() {
        if message
            .content
            .clone()
            .unwrap()
            .starts_with(&mate.prefix.clone().unwrap_or_default())
            && message
                .content
                .clone()
                .unwrap()
                .ends_with(&mate.postfix.clone().unwrap_or_default())
        {
            let message = ctx
                .http()
                .get_message(message.channel_id.0, message.id.0)
                .await?;
            send_proxied_message(
                ctx,
                &message,
                channel.clone(),
                mate,
                collective.clone(),
                database,
            )
            .await?;
            break;
        }
    }

    Ok(())
}

pub async fn on_message(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    let database = &data.database;

    let channels_collection = database.collection::<DBChannel>("channels");
    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");

    let dbchannel = channels_collection
        .find_one(doc! {"id": message.channel_id.0 as i64}, None)
        .await;

    let channel;

    if let Ok(Some(dbchannel)) = dbchannel {
        channel = dbchannel;
    } else {
        let guild_channel = message
            .channel(ctx.http())
            .await
            .context("Failed to get message's channel")?
            .guild()
            .context("Failed to get channel's guild channel")?;

        let webhook = guild_channel
            .id
            .create_webhook(ctx.http(), "Multiplex Proxier")
            .await;

        println!("{:?}", webhook);

        let webhook = webhook.context("Failed to create webhook")?;

        channel = DBChannel {
            id: guild_channel.id.0 as i64,
            webhook_id: webhook.id.0 as i64,
            webhook_token: webhook.token.unwrap(),
        };

        channels_collection
            .insert_one(channel.clone(), None)
            .await
            .context("Failed to write channel webhook to DB")?;
    }

    let mates = mates_collection
        .find(doc! {"user_id": message.author.id.0 as i64 }, None)
        .await
        .context("Failed to get user's mates")?;

    let mates = mates.try_collect::<Vec<DBMate>>().await?;

    let default_collective = DBCollective__new! {
        user_id = message.author.id.0 as i64,
        is_public = true,
    };

    let collective = collectives_collection
        .find_one(doc! { "user_id": message.author.id.0 as i64 }, None)
        .await
        .unwrap_or(Some(default_collective.clone()))
        .unwrap_or(default_collective);

    let mut did_proxy = false;

    for mate in mates.clone() {
        if message
            .content
            .starts_with(&mate.prefix.clone().unwrap_or_default())
            && message
                .content
                .ends_with(&mate.postfix.clone().unwrap_or_default())
        {
            send_proxied_message(
                ctx,
                message,
                channel.clone(),
                mate,
                collective.clone(),
                database,
            )
            .await?;

            did_proxy = true;
            break;
        }
    }

    if !did_proxy {
        for mate in mates {
            if mate.autoproxy {
                send_proxied_message(ctx, message, channel.clone(), mate, collective, database)
                    .await?;
                did_proxy = true;
                break;
            }
        }
    }

    Ok(())
}
