use orderless::{create_orderless, impl_orderless, make_orderless};

use serde::{Deserialize, Serialize};

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
