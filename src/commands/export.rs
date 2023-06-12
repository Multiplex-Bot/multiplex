use super::CommandContext;
use crate::models::{DBCollective, DBMate};
use crate::pluralkit::{Config, Member, MemberPrivacy, PluralkitExport, ProxyTag, SystemPrivacy};
use crate::utils;
use anyhow::Result;
use mongodb::bson::doc;
use poise::serenity_prelude::CreateAttachment;
use poise::CreateReply;

/// Export your collective to a format (theoretically) compatible with both Tupperbox and Pluralkit
#[poise::command(slash_command, ephemeral)]
pub async fn export(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    let collective =
        utils::get_or_create_collective(&collectives_collection, ctx.author().id).await?;

    let mates = utils::get_all_mates(&mates_collection, ctx.author().id).await?;

    let collective_privacy_str = if collective.is_public {
        "public"
    } else {
        "private"
    }
    .to_string();

    // FIXME: this is the worst function anybody has ever laid eyes on
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
                    last_message_timestamp: None,
                    id: "".to_string(),
                    uuid: "".to_string(),
                }
            })
            .collect::<Vec<Member>>(),
        tag: collective.collective_tag,
        // useless pluralkit garbage: part 2
        avatar_url: None,
        id: "".to_string(),
        uuid: "".to_string(),
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

    ctx.send(CreateReply::new()
        .content("Exported data! (Warning: This download may not work properly on mobile devices, because Discord doesn't know how to program.)").attachment(CreateAttachment::bytes(
            serde_json::to_vec(&export).unwrap(),
            "multiplex-export.json".to_string(),
        ))
    )
    .await?;

    Ok(())
}
