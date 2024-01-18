use anyhow::{Context, Result};
use mongodb::{bson::doc, Collection};
use poise::serenity_prelude::{
    ChannelId, CreateEmbed, CreateEmbedFooter, ExecuteWebhook, Http, Message,
};

use super::{channels::get_webhook_or_create, misc::envvar};
use crate::models::{DBChannel, DBGuild, DBMate};

pub async fn send_server_proxy_log(
    http: &Http,
    message: &Message,
    webhook_message: &Message,
    mate: DBMate,
    channels_collection: &Collection<DBChannel>,
    proxy_logs_channel_id: i64,
) -> Result<()> {
    let webhook = get_webhook_or_create(
        http,
        &channels_collection,
        // SAFETY: due to the chain of database-required type changes, this is fine to panic as `proxy_logs_channel_id` can never be zero
        ChannelId::new(proxy_logs_channel_id as u64),
    )
    .await?;

    let embed = CreateEmbed::new()
        .title(format!("Message proxied by `{}`", mate.name))
        .description(message.content.clone())
        .thumbnail(mate.avatar)
        .fields(vec![
            ("User", format!("<@{}>", message.author.id.get()), false),
            ("Proxied Message", webhook_message.link(), false),
        ])
        .footer(CreateEmbedFooter::new(format!(
            "Message ID: {} | Original message ID: {} | Channel ID: {} | User ID: {}",
            webhook_message.id.get(),
            message.id.get(),
            message.channel_id.get(),
            message.author.id
        )));

    let mut builder = ExecuteWebhook::new()
        .username("Multiplex (Proxy Logs)")
        .avatar_url(envvar("DEFAULT_AVATAR_URL"))
        .embed(embed);

    if let Some(thread_id) = webhook.1 {
        builder = builder.in_thread(thread_id);
    }

    webhook
        .0
        .execute(http, true, builder)
        .await?
        .context("Failed to send proxied message")?;

    Ok(())
}

pub async fn get_or_create_dbguild(
    collection: &Collection<DBGuild>,
    guild_id: i64,
) -> Result<DBGuild> {
    let guild = collection.find_one(doc! { "id": guild_id }, None).await?;

    if let Some(guild) = guild {
        Ok(guild)
    } else {
        let new_guild = DBGuild {
            id: guild_id,
            proxy_logs_channel_id: None,
            allowlist_role: None,
        };

        collection
            .insert_one(new_guild.clone(), None)
            .await
            .context("Failed to create new guild in database; try again later!")?;

        Ok(new_guild)
    }
}
