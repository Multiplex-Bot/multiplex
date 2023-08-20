use anyhow::Result;
use chrono::{DateTime, Utc};
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    models::{DBCollective, DBCollective__new, DBMate, DBMate__new},
    utils,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluralkitExport<'a> {
    pub version: i64,
    pub id: String,
    pub uuid: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tag: Option<String>,
    pub pronouns: Option<String>,
    pub avatar_url: Option<String>,
    pub banner: Option<String>,
    pub color: Option<String>,
    pub created: DateTime<Utc>,
    pub webhook_url: Option<String>,
    #[serde(borrow)]
    pub privacy: SystemPrivacy<'a>,
    pub config: Config,
    pub accounts: Vec<i64>,
    #[serde(borrow)]
    pub members: Vec<Member<'a>>,
    pub groups: Vec<Value>,
    pub switches: Vec<Switch>,
}

impl<'a> PluralkitExport<'a> {
    pub fn to_collective(&self, user_id: UserId) -> Result<DBCollective> {
        Ok(DBCollective__new! {
            user_id = user_id.0.get() as i64,
            is_public = !self.privacy.is_private()?,
            name = self.name.clone(),
            bio = self.description.clone(),
            pronouns = self.pronouns.clone(),
            collective_tag = self.tag.clone(),
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPrivacy<'a> {
    pub description_privacy: &'a str,
    pub pronoun_privacy: &'a str,
    pub member_list_privacy: &'a str,
    pub group_list_privacy: &'a str,
    pub front_privacy: &'a str,
    pub front_history_privacy: &'a str,
}

impl<'a> SystemPrivacy<'a> {
    pub fn is_private(&self) -> Result<bool> {
        Ok(serde_json::to_string(&self)?.contains("\"private\""))
    }

    pub fn create_from_single(privacy: &'a str) -> Self {
        SystemPrivacy::<'a> {
            description_privacy: privacy,
            pronoun_privacy: privacy,
            member_list_privacy: privacy,
            group_list_privacy: privacy,
            front_privacy: privacy,
            front_history_privacy: privacy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub timezone: String,
    pub pings_enabled: bool,
    pub latch_timeout: Option<i64>,
    pub member_default_private: bool,
    pub group_default_private: bool,
    pub show_private_info: bool,
    pub member_limit: i64,
    pub group_limit: i64,
    pub case_sensitive_proxy_tags: bool,
    pub description_templates: Vec<()>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
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
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member<'a> {
    pub id: String,
    pub uuid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
    pub birthday: Option<String>,
    pub pronouns: Option<String>,
    pub avatar_url: Option<String>,
    pub webhook_avatar_url: Option<String>,
    pub banner: Option<String>,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub keep_proxy: bool,
    pub autoproxy_enabled: bool,
    pub message_count: i64,
    pub last_message_timestamp: Option<String>,
    pub proxy_tags: Vec<ProxyTag>,
    #[serde(borrow)]
    pub privacy: MemberPrivacy<'a>,
}

impl<'a> Member<'a> {
    pub fn to_mate(&self, user_id: UserId) -> Result<DBMate> {
        let proxy_tags = self.proxy_tags.get(0).unwrap_or(&ProxyTag {
            prefix: None,
            suffix: None,
        });

        Ok(DBMate__new! {
                user_id = user_id.0.get() as i64,
                autoproxy = false,
                name = self.name.clone(),
                avatar = self
                    .avatar_url
                    .clone()
                    .unwrap_or(utils::envvar("DEFAULT_AVATAR_URL")),
                bio = self.description.clone(),
                prefix = proxy_tags.prefix.clone(),
                postfix = proxy_tags.suffix.clone(),
                pronouns = self.pronouns.clone(),
                display_name = self.display_name.clone(),
                is_public = !self.privacy.is_private()?,
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyTag {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberPrivacy<'a> {
    pub visibility: &'a str,
    pub name_privacy: &'a str,
    pub description_privacy: &'a str,
    pub birthday_privacy: &'a str,
    pub pronoun_privacy: &'a str,
    pub avatar_privacy: &'a str,
    pub metadata_privacy: &'a str,
}

impl<'a> MemberPrivacy<'a> {
    pub fn is_private(&self) -> Result<bool> {
        Ok(serde_json::to_string(&self)?.contains("\"private\""))
    }

    pub fn create_from_single(privacy: &'a str) -> Self {
        MemberPrivacy::<'a> {
            description_privacy: privacy,
            pronoun_privacy: privacy,
            avatar_privacy: privacy,
            birthday_privacy: privacy,
            metadata_privacy: privacy,
            name_privacy: privacy,
            visibility: privacy,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Switch {
    pub timestamp: String,
    pub members: Vec<String>,
}
