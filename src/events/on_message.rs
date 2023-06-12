use anyhow::{Context, Result};
use mongodb::bson::doc;
use poise::futures_util::TryStreamExt;

use poise::serenity_prelude::{
    CacheHttp, ChannelType, Context as SerenityContext, CreateWebhook, Message,
};

use crate::commands::Data;
use crate::models::{DBChannel, DBCollective, DBCollective__new, DBMate};

use crate::utils::send_proxied_message;

pub async fn run(ctx: &SerenityContext, data: &Data, message: &Message) -> Result<()> {
    let database = &data.database;

    let channels_collection = database.collection::<DBChannel>("channels");
    let mates_collection = database.collection::<DBMate>("mates");
    let collectives_collection = database.collection::<DBCollective>("collectives");

    let mut thread_id = None;

    let mut guild_channel = message
        .channel(ctx.http())
        .await
        .context("Failed to get message's channel")?
        .guild()
        .context("Failed to get channel's guild channel")?;

    if guild_channel.kind == ChannelType::PublicThread
        || guild_channel.kind == ChannelType::PrivateThread
    {
        guild_channel = ctx
            .http()
            .get_channel(guild_channel.parent_id.unwrap())
            .await
            .context("Failed to get thread's channel")?
            .guild()
            .context("Failed to get channel's guild channel")?;
        thread_id = Some(message.channel_id);
    }

    let dbchannel = channels_collection
        .find_one(doc! {"id": guild_channel.id.0.get() as i64}, None)
        .await;

    let channel;

    if let Ok(Some(dbchannel)) = dbchannel {
        channel = dbchannel;
    } else {
        if let Some(_) = message
            .channel(ctx.http())
            .await
            .context("Failed to get message's channel")?
            .private()
        {
            return Ok(());
        }

        let webhook = guild_channel
            .id
            .create_webhook(ctx.http(), CreateWebhook::new("Multiplex Proxier"))
            .await;

        println!("{:?}", webhook);

        let webhook = webhook.context("Failed to create webhook")?;

        channel = DBChannel {
            id: guild_channel.id.0.get() as i64,
            webhook_id: webhook.id.0.get() as i64,
            webhook_token: webhook.token.unwrap(),
        };

        channels_collection
            .insert_one(channel.clone(), None)
            .await
            .context("Failed to write channel webhook to DB")?;
    }

    let mates = mates_collection
        .find(doc! {"user_id": message.author.id.0.get() as i64 }, None)
        .await
        .context("Failed to get user's mates")?;

    let mates = mates.try_collect::<Vec<DBMate>>().await?;

    let default_collective = DBCollective__new! {
        user_id = message.author.id.0.get() as i64,
        is_public = true,
    };

    let collective = collectives_collection
        .find_one(doc! { "user_id": message.author.id.0.get() as i64 }, None)
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
            send_proxied_message(ctx.http(), message, mate, collective.clone(), database).await?;

            did_proxy = true;
            break;
        }
    }

    if !did_proxy {
        for mate in mates {
            if mate.autoproxy {
                send_proxied_message(ctx.http(), message, mate, collective.clone(), database)
                    .await?;
                break;
            }
        }
    }

    Ok(())
}
