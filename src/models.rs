use anyhow::{Context, Result};
use mongodb::{bson::doc, Collection};
use orderless::impl_orderless;
use poise::serenity_prelude::Http;
use serde::{Deserialize, Serialize};

use crate::utils::{parse_selector, upload_avatar};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DBMate {
    pub user_id: i64,
    pub autoproxy: bool,
    pub name: String,
    pub avatar: String,
    pub is_public: bool,
    pub bio: Option<String>,
    pub prefix: Option<String>,
    pub postfix: Option<String>,
    pub pronouns: Option<String>,
    pub display_name: Option<String>,
}

#[impl_orderless]
impl DBMate {
    #[make_orderless(
        public = true,
        defs(bio = None, prefix = None, postfix = None, pronouns = None, display_name = None),
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
        }
    }

    pub async fn edit(
        &mut self,
        http: &Http,
        collection: Collection<DBMate>,
        name: Option<String>,
        display_name: Option<String>,
        bio: Option<String>,
        pronouns: Option<String>, // where d Joe get those proNOUNS fro m,,,,  ? The pronoun sto are 🤣🤣🤣  ?? !!,,, Their gender ? (/j)
        selector: Option<String>,
        publicity: Option<bool>,
        avatar: Option<poise::serenity_prelude::Attachment>,
    ) -> Result<()> {
        let current_name = self.name.clone();

        if let Some(name) = name {
            self.name = name
        }

        if let Some(display_name) = display_name {
            self.display_name = Some(display_name)
        }

        if let Some(bio) = bio {
            self.bio = Some(bio)
        }

        if let Some(pronouns) = pronouns {
            self.pronouns = Some(pronouns)
        }

        let (prefix, postfix) = parse_selector(selector);
        if let Some(prefix) = prefix {
            self.prefix = Some(prefix)
        }
        if let Some(postfix) = postfix {
            self.postfix = Some(postfix)
        }

        if let Some(publicity) = publicity {
            self.is_public = publicity
        }

        if let Some(avatar) = avatar {
            self.avatar = upload_avatar(http, avatar).await?;
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
    pub user_id: i64,
    pub is_public: bool,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub pronouns: Option<String>,
    pub collective_tag: Option<String>,
}

#[impl_orderless]
impl DBCollective {
    #[make_orderless(
        public = true,
        defs(name = None, bio = None, pronouns = None, collective_tag = None),
    )]
    pub fn new(
        user_id: i64,
        is_public: bool,
        name: Option<String>,
        bio: Option<String>,
        pronouns: Option<String>,
        collective_tag: Option<String>,
    ) -> DBCollective {
        DBCollective {
            user_id,
            is_public,
            name,
            bio,
            pronouns,
            collective_tag,
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
        if let Some(name) = name {
            self.name = Some(name)
        }

        if let Some(bio) = bio {
            self.bio = Some(bio)
        }

        if let Some(pronouns) = pronouns {
            self.pronouns = Some(pronouns)
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
pub struct DBChannel {
    pub id: i64,
    pub webhook_id: i64,
    pub webhook_token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBMessage {
    pub user_id: u64,
    pub message_id: u64,
}
