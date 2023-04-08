use anyhow::{anyhow, Context, Result};
use mongodb::bson::{doc, Document};
use poise::futures_util::{StreamExt, TryStreamExt};
use poise::serenity_prelude::{CacheHttp, Context as SerenityContext, Message, Webhook, WebhookId};

use crate::commands::Data;
use crate::models::{DBChannel, DBMate};

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

    for mate in mates {
        if (message
            .content
            .starts_with(&mate.prefix.clone().unwrap_or_default())
            && message
                .content
                .ends_with(&mate.postfix.clone().unwrap_or_default()))
            || mate.autoproxy
        {
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

            webhook
                .execute(ctx.http(), false, |msg| {
                    msg.avatar_url(mate.avatar)
                        .username(mate.name)
                        .content(new_content)
                })
                .await?;
            message.delete(ctx.http()).await?;
        }
    }

    Ok(())
}
