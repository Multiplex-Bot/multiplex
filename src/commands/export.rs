use super::CommandContext;
use crate::models::{DBCollective, DBMate};
use crate::pluralkit::{Config, Member, MemberPrivacy, PluralkitExport, ProxyTag, SystemPrivacy};
use anyhow::{Context, Error, Result};
use mongodb::bson::{self, doc};
use poise::futures_util::TryStreamExt;
use poise::serenity_prelude::{self, Attachment, CacheHttp};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    // Should always be `2`
    pub version: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub pronouns: Option<String>,
    pub privacy: SystemPrivacy,
    pub members: Vec<MateExport>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MateExport {
    pub name: String,
    pub display_name: Option<String>,
    pub pronouns: Option<String>,
    pub avatar_url: Option<String>,
    pub description: Option<String>,
    pub proxy_tags: Vec<ProxyTag>,
    pub privacy: MemberPrivacy,
}

/// Export your collective to a format (theoretically) compatible with both Tupperbox and Pluralkit
#[poise::command(slash_command)]
pub async fn export(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    let default_collective = DBCollective {
        user_id: ctx.author().id.0 as i64,
        name: None,
        bio: None,
        pronouns: None,
        is_public: true,
    };

    let collective = collectives_collection
        .find_one(doc! { "user_id": ctx.author().id.0 as i64 }, None)
        .await
        // NOTE: I've seen it error on both of these whenever there's not a result for the query, so I'm not sure which it actually should be
        .unwrap_or(Some(default_collective.clone()))
        .unwrap_or(default_collective);

    let mates = mates_collection
        .find(doc! {"user_id": ctx.author().id.0 as i64 }, None)
        .await
        .context("Failed to get user's mates")?;

    let mates = mates.try_collect::<Vec<DBMate>>().await?;

    let collective_privacy_str = if collective.is_public {
        "public"
    } else {
        "private"
    }
    .to_string();

    let export = PluralkitExport {
        version: 2,
        name: collective.name,
        description: collective.bio,
        pronouns: collective.pronouns,
        privacy: SystemPrivacy {
            description_privacy: collective_privacy_str.clone(),
            pronoun_privacy: collective_privacy_str.clone(),
            member_list_privacy: collective_privacy_str.clone(),
            group_list_privacy: collective_privacy_str.clone(),
            front_privacy: collective_privacy_str.clone(),
            front_history_privacy: collective_privacy_str,
        },
        members: mates
            .iter()
            .map(|mate| {
                let privacy_str = if mate.is_public { "public" } else { "private" }.to_string();
                Member {
                    name: mate.name.clone(),
                    display_name: mate.display_name.clone(),
                    pronouns: mate.pronouns.clone(),
                    avatar_url: Some(mate.avatar.clone()),
                    description: mate.bio.clone(),
                    proxy_tags: vec![ProxyTag {
                        prefix: mate.prefix.clone(),
                        suffix: mate.postfix.clone(),
                    }],
                    privacy: MemberPrivacy {
                        visibility: privacy_str.clone(),
                        name_privacy: privacy_str.clone(),
                        description_privacy: privacy_str.clone(),
                        birthday_privacy: privacy_str.clone(),
                        pronoun_privacy: privacy_str.clone(),
                        avatar_privacy: privacy_str.clone(),
                        metadata_privacy: privacy_str,
                    },
                    // useless pluralkit garbage
                    autoproxy_enabled: true,
                    keep_proxy: false,
                    banner: None,
                    birthday: None,
                    color: None,
                    webhook_avatar_url: None,
                    created: "".to_string(),
                    message_count: 0,
                    last_message_timestamp: "".to_string(),
                    id: "".to_string(),
                    uuid: "".to_string(),
                }
            })
            .collect::<Vec<Member>>(),
        // useless pluralkit garbage
        avatar_url: None,
        id: "".to_string(),
        uuid: "".to_string(),
        tag: None,
        banner: None,
        color: None,
        created: "".to_string(),
        webhook_url: None,
        config: Config {
            timezone: "UTC".to_string(),
            pings_enabled: false,
            case_sensitive_proxy_tags: true,
            latch_timeout: None,
            member_default_private: false,
            show_private_info: false,
            group_default_private: false,
            member_limit: 1000,
            group_limit: 250,
            description_templates: vec![],
        },
        accounts: vec![],
        groups: vec![],
        switches: vec![],
    };

    ctx.send(|b| {
        b.attachment(serenity_prelude::AttachmentType::Bytes {
            data: serde_json::to_vec(&export).unwrap().into(),
            filename: "multiplex-export.json".to_string(),
        })
    })
    .await?;

    Ok(())
}
