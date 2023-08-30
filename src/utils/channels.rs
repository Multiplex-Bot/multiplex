use std::num::NonZeroU64;

use anyhow::{Context, Result};
use mongodb::{bson::doc, Collection};
use poise::serenity_prelude::{ChannelId, CreateWebhook, Http, Webhook, WebhookId};
use secrecy::ExposeSecret;

use super::misc::is_thread;
use crate::models::DBChannel;

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
            webhook_token: webhook.token.as_ref().unwrap().expose_secret().clone(),
        };

        collection
            .insert_one(channel.clone(), None)
            .await
            .context("Failed to write channel webhook to DB")?;
    }

    Ok((webhook, thread_id))
}
