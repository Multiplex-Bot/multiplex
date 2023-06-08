use crate::models::{DBCollective, DBCollective__new, DBMate, DBMate__new};
use anyhow::Result;
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluralkitExport {
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
    pub created: String,
    pub webhook_url: Option<String>,
    pub privacy: SystemPrivacy,
    pub config: Config,
    pub accounts: Vec<i64>,
    pub members: Vec<Member>,
    pub groups: Vec<Value>,
    pub switches: Vec<Switch>,
}

impl PluralkitExport {
    pub fn to_collective(&self, user_id: UserId) -> Result<DBCollective> {
        Ok(DBCollective__new! {
            user_id = user_id.0.get() as i64,
            is_public = !serde_json::to_string(&self.privacy)?.contains("\"private\""),
            name = self.name.clone(),
            bio = self.description.clone(),
            pronouns = self.pronouns.clone(),
            collective_tag = self.tag.clone(),
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPrivacy {
    pub description_privacy: String,
    pub pronoun_privacy: String,
    pub member_list_privacy: String,
    pub group_list_privacy: String,
    pub front_privacy: String,
    pub front_history_privacy: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
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
    pub created: String,
    pub keep_proxy: bool,
    pub autoproxy_enabled: bool,
    pub message_count: i64,
    pub last_message_timestamp: Option<String>,
    pub proxy_tags: Vec<ProxyTag>,
    pub privacy: MemberPrivacy,
}

impl Member {
    pub fn to_mate(&self, user_id: UserId) -> Result<DBMate> {
        Ok(DBMate__new! {
                user_id = user_id.0.get() as i64,
                autoproxy = false,
                name = self.name.clone(),
                avatar = self
                    .avatar_url
                    .clone()
                    .unwrap_or(std::env::var("DEFAULT_AVATAR_URL").unwrap()),
                bio = self.description.clone(),
                prefix = self.proxy_tags[0].prefix.clone(),
                postfix = self.proxy_tags[0].suffix.clone(),
                pronouns = self.pronouns.clone(),
                display_name = self.display_name.clone(),
                is_public = !serde_json::to_string(&self.privacy)?.contains("\"private\""),
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyTag {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberPrivacy {
    pub visibility: String,
    pub name_privacy: String,
    pub description_privacy: String,
    pub birthday_privacy: String,
    pub pronoun_privacy: String,
    pub avatar_privacy: String,
    pub metadata_privacy: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Switch {
    pub timestamp: String,
    pub members: Vec<String>,
}
