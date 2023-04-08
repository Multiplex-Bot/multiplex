use anyhow::{anyhow, Context, Result};
use mongodb::bson::{doc, Document};
use poise::futures_util::{StreamExt, TryStreamExt};
use poise::serenity_prelude::json::Value;
use poise::serenity_prelude::{
    CacheHttp, Context as SerenityContext, Embed, Message, MessageType, Webhook, WebhookId,
};

use crate::commands::Data;
use crate::models::{DBChannel, DBMate};

async fn send_proxied_message(
    ctx: &SerenityContext,
    message: &Message,
    channel: DBChannel,
    mate: DBMate,
) -> Result<()> {
    let webhook = Webhook::from_id_with_token(
        ctx.http(),
        WebhookId(channel.webhook_id as u64),
        &channel.webhook_token,
    )
    .await?;

    let mut new_content = message.content.clone();

    if !mate.autoproxy {
        new_content = new_content
            .strip_prefix(&mate.prefix.clone().unwrap_or_default())
            .unwrap()
            .strip_suffix(&mate.postfix.clone().unwrap_or_default())
            .unwrap()
            .to_string();
    }

    if let Some(referenced_message) = &message.referenced_message {
        // FIXME: this whole reply system is *really* jank, and will break if anyone uses a zero-width space in a message.
        new_content = format!(
            "> {}\n[jump to content](https://discord.com/channels/{}/{}/{}) {}\n​{}​",
            if let Some(_) = referenced_message.webhook_id {
                let mut message_parts = referenced_message
                    .content
                    .split("​")
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                message_parts.reverse();

                message_parts
                    .get(1)
                    .unwrap_or(&referenced_message.content)
                    .replace("\n", "\n> ")
            } else {
                referenced_message.content.replace("\n", "\n> ")
            },
            message.guild_id.unwrap().0,
            referenced_message.channel_id.0,
            referenced_message.id.0,
            if message.mentions_user(&referenced_message.author) {
                if let Some(_) = referenced_message.webhook_id {
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

    webhook
        .execute(ctx.http(), false, |msg| {
            msg.avatar_url(mate.avatar)
                .username(if let Some(display_name) = mate.display_name {
                    display_name
                } else {
                    mate.name
                })
                .content(new_content)
        })
        .await?;
    message.delete(ctx.http()).await?;

    Ok(())
}

pub async fn on_message(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    let database = &data.database;

    let channels_collection = database.collection::<DBChannel>("channels");
    let mates_collection = database.collection::<DBMate>("mates");

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
        .find(doc! {"user_id": message.author.id.0 as i64 }, None)
        .await
        .context("Failed to get user's mates")?;

    let mates = mates.try_collect::<Vec<DBMate>>().await?;

    let mut did_proxy = false;

    for mate in mates.clone() {
        if message
            .content
            .starts_with(&mate.prefix.clone().unwrap_or_default())
            && message
                .content
                .ends_with(&mate.postfix.clone().unwrap_or_default())
        {
            send_proxied_message(ctx, message, channel.clone(), mate).await?;

            did_proxy = true;
            break;
        }
    }

    if !did_proxy {
        for mate in mates {
            if mate.autoproxy {
                send_proxied_message(ctx, message, channel.clone(), mate).await?;
                break;
            }
        }
    }

    Ok(())
}
