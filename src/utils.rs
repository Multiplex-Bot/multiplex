use std::{borrow::Cow, env, num::NonZeroU64};

use anyhow::{bail, Context, Result};
use mime2ext::mime2ext;
use mongodb::{
    bson::{self, doc},
    options::FindOneOptions,
    results::DeleteResult,
    Collection, Database,
};
use poise::{
    futures_util::TryStreamExt,
    serenity_prelude::{
        Attachment, ChannelId, ChannelType, CreateAttachment, CreateEmbed, CreateEmbedAuthor,
        CreateEmbedFooter, CreateWebhook, ExecuteWebhook, GuildChannel, GuildId, Http, Message,
        MessageId, UserId, Webhook, WebhookId,
    },
};
use s3::Bucket;
use secrecy::ExposeSecret;
use unicode_segmentation::UnicodeSegmentation;
use urlencoding::encode;

use crate::models::{
    AutoproxySettings, DBChannel, DBCollective, DBCollective__new, DBGuild, DBMate, DBMessage,
    DBUserSettings, Latch,
};

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

pub async fn get_or_create_user_settings(
    collection: &Collection<DBUserSettings>,
    user_id: UserId,
    guild_id: Option<i64>,
) -> Result<DBUserSettings> {
    fn fallback(settings: &mut DBUserSettings, fallback_settings: DBUserSettings) {
        if settings.autoproxy.is_none() {
            settings.autoproxy = fallback_settings.autoproxy;
        }
    }

    let settings = collection
        .find_one(
            doc! { "user_id": user_id.get() as i64, "guild_id": guild_id },
            None,
        )
        .await?;

    if let Some(mut settings) = settings {
        if settings.guild_id.is_some() {
            let user_settings = collection
                .find_one(
                    doc! { "user_id": user_id.get() as i64, "guild_id": None::<i64> },
                    None,
                )
                .await;

            if let Ok(Some(user_settings)) = user_settings {
                fallback(&mut settings, user_settings);
            }
        }

        Ok(settings)
    } else {
        let mut new_settings = DBUserSettings {
            user_id: user_id.get(),
            autoproxy: if guild_id.is_some() {
                None
            } else {
                Some(AutoproxySettings::SwitchedIn)
            },
            guild_id: guild_id,
        };

        collection
            .insert_one(&new_settings, None)
            .await
            .context("Failed to create new user settings in database; try again later!")?;

        if guild_id.is_some() {
            let user_settings = collection
                .find_one(
                    doc! { "user_id": user_id.get() as i64, "guild_id": None::<i64> },
                    None,
                )
                .await;

            if let Ok(Some(user_settings)) = user_settings {
                fallback(&mut new_settings, user_settings);
            } else {
                let new_user_settings = DBUserSettings {
                    user_id: user_id.get(),
                    autoproxy: Some(AutoproxySettings::SwitchedIn),
                    guild_id: None,
                };

                collection
                    .insert_one(&new_user_settings, None)
                    .await
                    .context("Failed to create new user settings in database; try again later!")?;

                fallback(&mut new_settings, new_user_settings);
            }
        }

        Ok(new_settings)
    }
}

pub async fn update_user_settings(
    collection: &Collection<DBUserSettings>,
    settings: DBUserSettings,
    autoproxy: Option<AutoproxySettings>,
) -> Result<()> {
    let mut new_settings = settings.clone();

    if autoproxy.is_some() {
        new_settings.autoproxy = autoproxy;
    }

    collection
        .update_one(
            doc! {
                "user_id": new_settings.user_id as i64,
                "guild_id": new_settings.guild_id
            },
            doc! { "$set": bson::to_bson(&new_settings).unwrap() },
            None,
        )
        .await?;

    Ok(())
}

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

pub async fn upload_avatar(
    avatar_bucket: &Bucket,
    user_id: UserId,
    mate_name: String,
    attachment: Attachment,
) -> Result<String> {
    let file_ext = mime2ext(attachment.content_type.clone().context(
        "The file does not have a mime type; this should not be possible. What arcane magic did \
         you use?",
    )?)
    .context("Failed to convert detected file type to extension!")?;

    avatar_bucket
        .put_object(
            format!("/{}/{}.{}", user_id.get(), mate_name, file_ext),
            &*attachment.download().await?,
        )
        .await?;

    Ok(format!(
        "{}/{}/{}.{}",
        envvar("PUBLIC_AVATAR_URL"),
        user_id.get(),
        encode(&mate_name),
        file_ext
    ))
}

pub fn is_thread(channel: &GuildChannel) -> bool {
    channel.kind == ChannelType::PublicThread || channel.kind == ChannelType::PrivateThread
}

pub fn get_matching_mate<'a>(
    mates: &'a Vec<DBMate>,
    message_content: &String,
) -> Option<&'a DBMate> {
    for mate in mates {
        // account for proxy-tag-less mates
        if mate.prefix.is_some() || mate.postfix.is_some() {
            if message_content.starts_with(&mate.prefix.clone().unwrap_or_default())
                && message_content.ends_with(&mate.postfix.clone().unwrap_or_default())
            {
                return Some(mate);
            }
        }
    }
    None
}

pub async fn get_autoproxied_mate<'a>(
    settings_collection: &Collection<DBUserSettings>,
    mates: &'a Vec<DBMate>,
    user_id: UserId,
    guild_id: GuildId,
) -> Option<&'a DBMate> {
    let Ok(user_settings) =
        get_or_create_user_settings(settings_collection, user_id, Some(guild_id.get() as i64)).await
            else { return None };

    match user_settings.autoproxy {
        Some(AutoproxySettings::Disabled) => None,
        Some(AutoproxySettings::SwitchedIn) => {
            for mate in mates {
                if mate.autoproxy {
                    return Some(mate);
                }
            }
            None
        }
        Some(AutoproxySettings::Latch(latch)) => {
            let mate_name = match latch {
                Latch::Guild(Some(guild)) => {
                    // not sure if this is actually needed
                    if user_settings.guild_id == Some(guild_id.get() as i64) {
                        Some(guild)
                    } else {
                        None
                    }
                }
                Latch::Global(Some(global)) => Some(global),
                _ => None,
            }?;

            Some(mates.iter().filter(|mate| mate.name == mate_name).next()?)
        }
        Some(AutoproxySettings::Mate(mate_name)) => {
            Some(mates.iter().filter(|mate| mate.name == mate_name).next()?)
        }
        None => None,
    }
}

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
        // SAFETY: due to the chain of database-required type changes, this is fine to unwrap as `proxy_logs_channel_id` can never be zero
        ChannelId(NonZeroU64::new(proxy_logs_channel_id as u64).unwrap()),
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

pub async fn update_latch(
    settings_collection: &Collection<DBUserSettings>,
    message: &Message,
    new: Option<String>,
) -> Result<()> {
    let guild_settings = get_or_create_user_settings(
        &settings_collection,
        message.author.id,
        message
            .guild_id
            .and_then(|guild_id| Some(guild_id.get() as i64)),
    )
    .await?;

    match guild_settings.autoproxy {
        Some(AutoproxySettings::Latch(Latch::Global(_))) => {
            let global_settings =
                get_or_create_user_settings(&settings_collection, message.author.id, None).await?;

            update_user_settings(
                &settings_collection,
                global_settings,
                Some(AutoproxySettings::Latch(Latch::Global(new))),
            )
            .await?;
        }
        Some(AutoproxySettings::Latch(Latch::Guild(_))) => {
            update_user_settings(
                &settings_collection,
                guild_settings,
                Some(AutoproxySettings::Latch(Latch::Guild(new))),
            )
            .await?;
        }
        _ => {}
    }

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
        };

        collection
            .insert_one(new_guild.clone(), None)
            .await
            .context("Failed to create new guild in database; try again later!")?;

        Ok(new_guild)
    }
}

pub async fn update_guild_settings(
    collection: &Collection<DBGuild>,
    guild: DBGuild,
    proxy_logs_channel_id: Option<i64>,
) -> Result<()> {
    let mut new_guild = guild.clone();

    if proxy_logs_channel_id.is_some() {
        new_guild.proxy_logs_channel_id = proxy_logs_channel_id;
    }

    collection
        .update_one(
            doc! {
                "id": new_guild.id
            },
            doc! { "$set": bson::to_bson(&new_guild).unwrap() },
            None,
        )
        .await?;

    Ok(())
}

pub fn envvar(var: &str) -> String {
    env::var(var).expect(&format!(
        "Could not find {}; did you specify it in .env?",
        var
    ))
}
