use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};
use orderless::impl_orderless;
use serde::{Deserialize, Serialize};

use crate::utils::messages::parse_selector;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DBMate {
    #[serde(rename = "_id", skip_serializing)]
    pub id: Option<ObjectId>,
    pub user_id: i64,
    pub autoproxy: bool,
    pub name: String,
    pub avatar: String,
    pub is_public: bool,
    pub bio: Option<String>,
    pub prefix: Option<String>,
    pub postfix: Option<String>,
    pub pronouns: Option<String>,
    pub signature: Option<Signature>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Signature {
    pub prefix: String,
    pub postfix: String,
}

impl Signature {
    pub fn parse(signature: String) -> Self {
        let mut split_sig = signature.split("text");

        let split_sig = (split_sig.next(), split_sig.next());

        let split_sig = match split_sig {
            (None, None) => (String::new(), String::new()),
            (None, Some(r)) => (String::new(), r.to_string()),
            (Some(l), None) => (l.to_string(), String::new()),
            (Some(l), Some(r)) => (l.to_string(), r.to_string()),
        };

        Signature {
            prefix: split_sig.0,
            postfix: split_sig.1,
        }
    }
}

#[impl_orderless]
impl DBMate {
    #[make_orderless(
        public = true,
        defs(bio = None, prefix = None, postfix = None, pronouns = None, display_name = None, signature = None, id = None),
    )]
    pub fn new(
        user_id: i64,
        autoproxy: bool,
        name: String,
        avatar: String,
        is_public: bool,
        bio: Option<String>,
        prefix: Option<String>,
        postfix: Option<String>,
        pronouns: Option<String>,
        display_name: Option<String>,
        signature: Option<Signature>,
        id: Option<ObjectId>,
    ) -> DBMate {
        DBMate {
            user_id,
            autoproxy,
            name,
            avatar,
            is_public,
            bio,
            prefix,
            postfix,
            pronouns,
            display_name,
            signature,
            id,
        }
    }

    pub async fn edit(
        &mut self,
        collection: Collection<DBMate>,
        name: Option<String>,
        display_name: Option<String>,
        bio: Option<String>,
        pronouns: Option<String>, // where d Joe get those proNOUNS fro m,,,,  ? The pronoun sto are 🤣🤣🤣  ?? !!,,, Their gender ? (/j)
        selector: Option<String>,
        publicity: Option<bool>,
        avatar: Option<String>,
        signature: Option<String>,
    ) -> Result<()> {
        let current_name = self.name.clone();

        if let Some(name) = name {
            self.name = name
        }

        if display_name.is_some() {
            self.display_name = display_name
        }

        if bio.is_some() {
            self.bio = bio
        }

        if pronouns.is_some() {
            self.pronouns = pronouns
        }

        let (prefix, postfix) = parse_selector(selector);
        if prefix.is_some() {
            self.prefix = prefix;
        }
        if postfix.is_some() {
            self.postfix = postfix;
        }

        if let Some(publicity) = publicity {
            self.is_public = publicity
        }

        if let Some(avatar) = avatar {
            self.avatar = avatar;
        }

        if let Some(signature) = signature {
            self.signature = Some(Signature::parse(signature))
        }

        collection
            .find_one_and_replace(
                doc! { "user_id": self.user_id, "name": current_name },
                self,
                None,
            )
            .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBCollective {
    #[serde(rename = "_id", skip_serializing)]
    pub id: Option<ObjectId>,
    pub user_id: i64,
    pub is_public: bool,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub pronouns: Option<String>,
    pub collective_tag: Option<String>,
    pub switch_logs: Option<Vec<SwitchLog>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwitchLog {
    pub date: DateTime<Utc>,
    pub mate_id: Option<ObjectId>,
    pub previous_mate_id: Option<ObjectId>,
    pub unswitch: bool,
}

#[impl_orderless]
impl DBCollective {
    #[make_orderless(
        public = true,
        defs(name = None, bio = None, pronouns = None, collective_tag = None, id = None, switch_logs = None),
    )]
    pub fn new(
        user_id: i64,
        is_public: bool,
        name: Option<String>,
        bio: Option<String>,
        pronouns: Option<String>,
        collective_tag: Option<String>,
        id: Option<ObjectId>,
        switch_logs: Option<Vec<SwitchLog>>,
    ) -> DBCollective {
        DBCollective {
            user_id,
            is_public,
            name,
            bio,
            pronouns,
            collective_tag,
            id,
            switch_logs,
        }
    }

    pub async fn edit(
        &mut self,
        collection: Collection<DBCollective>,
        name: Option<String>,
        bio: Option<String>,
        pronouns: Option<String>,
        is_public: Option<bool>,
        collective_tag: Option<String>,
    ) -> Result<()> {
        if name.is_some() {
            self.name = name
        }

        if bio.is_some() {
            self.bio = bio
        }

        if pronouns.is_some() {
            // WHRERED JOE GED THEIR PRONOUBCE FROM ??! THE PRONONNCE ORE 🖐️🖐️🖐️🙄🙄🙄
            self.pronouns = pronouns;
        }

        if let Some(is_public) = is_public {
            self.is_public = is_public
        }

        if let Some(collective_tag) = collective_tag {
            if collective_tag == "" {
                self.collective_tag = None
            } else {
                self.collective_tag = Some(collective_tag)
            }
        }

        collection
            .find_one_and_replace(doc! { "user_id": self.user_id }, self, None)
            .await?
            .context("Failed to update collective information; try again later!")?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBGuild {
    pub id: i64,
    pub proxy_logs_channel_id: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBChannel {
    pub id: i64,
    pub webhook_id: i64,
    pub webhook_token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBMessage {
    pub user_id: u64,
    pub message_id: u64,
    pub mate_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutoproxySettings {
    Disabled,
    SwitchedIn,
    Latch(Latch),
    Mate(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBUserSettings {
    pub user_id: u64,
    pub guild_id: Option<i64>,
    pub autoproxy: Option<AutoproxySettings>,
    pub regex_sed_editing: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Latch {
    Guild(Option<String>),
    Global(Option<String>),
}
