use anyhow::Result;
use mongodb::bson::doc;
use poise::{serenity_prelude::CreateAttachment, CreateReply};

use super::CommandContext;
use crate::{
    models::{DBCollective, DBMate},
    pluralkit::{Member, MemberPrivacy, PluralkitExport, ProxyTag, SystemPrivacy},
    utils::{collectives::get_or_create_collective, mates::get_all_mates},
};

/// Export your collective to a format (theoretically) compatible with both Tupperbox and Pluralkit
#[poise::command(slash_command, ephemeral)]
pub async fn export(ctx: CommandContext<'_>) -> Result<()> {
    let database = &ctx.data().database;

    let collectives_collection = database.collection::<DBCollective>("collectives");
    let mates_collection = database.collection::<DBMate>("mates");

    let collective = get_or_create_collective(&collectives_collection, ctx.author().id).await?;

    let mates = get_all_mates(&mates_collection, ctx.author().id).await?;

    let collective_privacy_str = if collective.is_public {
        "public"
    } else {
        "private"
    };

    let export = PluralkitExport {
        version: 2,
        name: collective.name,
        description: collective.bio,
        // impossible for the id to not exist
        created: collective.id.unwrap().timestamp().to_chrono(),
        pronouns: collective.pronouns,
        privacy: SystemPrivacy::create_from_single(collective_privacy_str),
        members: mates
            .iter()
            .map(|mate| {
                let privacy_str = if mate.is_public { "public" } else { "private" };

                Member {
                    name: mate.name.clone(),
                    display_name: mate.display_name.clone(),
                    pronouns: mate.pronouns.clone(),
                    avatar_url: Some(mate.avatar.clone()),
                    description: mate.bio.clone(),
                    // impossible for the id to not exist
                    created: mate.id.unwrap().timestamp().to_chrono(),
                    proxy_tags: vec![ProxyTag {
                        prefix: mate.prefix.clone(),
                        suffix: mate.postfix.clone(),
                    }],
                    privacy: MemberPrivacy::create_from_single(privacy_str),
                    // useless pluralkit garbage
                    autoproxy_enabled: true,
                    keep_proxy: false,
                    banner: None,
                    birthday: None,
                    color: None,
                    webhook_avatar_url: None,
                    message_count: 0,
                    last_message_timestamp: None,
                    id: Default::default(),
                    uuid: Default::default(),
                }
            })
            .collect::<Vec<Member>>(),
        tag: collective.collective_tag,
        // useless pluralkit garbage: part 2
        avatar_url: None,
        id: Default::default(),
        uuid: Default::default(),
        banner: None,
        color: None,
        webhook_url: None,
        config: Default::default(),
        accounts: vec![],
        groups: vec![],
        switches: vec![],
    };

    ctx.send(
        CreateReply::new()
            .content(
                "Exported data! (Warning: This download may not work properly on mobile devices, \
                 because Discord doesn't know how to program.)",
            )
            .attachment(CreateAttachment::bytes(
                serde_json::to_vec(&export).unwrap(),
                "multiplex-export.json".to_string(),
            )),
    )
    .await?;

    Ok(())
}
