use std::env;

use anyhow::{Context, Result};
use mime2ext::mime2ext;
use poise::serenity_prelude::{Attachment, ChannelType, GuildChannel, MessageId, UserId};
use s3::Bucket;
use urlencoding::encode;

/// note: this should have the `s/` passed into it as a prefix
pub fn handle_sed_edit(_message_content: &String, sed_statement: &String) {
    let mut i = 0;
    let statement = sed_statement
        .split(|character| {
            let res = character == '/' && sed_statement.chars().nth(i - 1).unwrap_or(' ') != '\\';
            i += 1;
            return res;
        })
        .collect::<Vec<&str>>();

    if statement[0] == "s" {
    } else {
        // not a sed statement idiot stupid goofy ahh mother Fucked
    }
}

pub fn envvar(var: &str) -> String {
    env::var(var).expect(&format!(
        "Could not find {}; did you specify it in .env?",
        var
    ))
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

pub fn message_link_to_id(message_link: String) -> Result<MessageId> {
    let iter = message_link.split("/");
    let message_id = iter.last().context("Failed to get message ID from link!")?;

    Ok(MessageId::new(message_id.parse::<u64>()?))
}
