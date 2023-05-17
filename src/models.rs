use orderless::create_orderless;

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

impl DBMate {
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

create_orderless! {
    public = true,
    func = DBMate::new,
    defs(user_id, autoproxy, name, avatar, is_public, bio = None, prefix = None, postfix = None, pronouns = None, display_name = None),
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

impl DBCollective {
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

create_orderless! {
    public = true,
    func = DBCollective::new,
    defs(user_id, is_public, name = None, bio = None, pronouns = None, collective_tag = None),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DBChannel {
    pub id: i64,
    pub webhook_id: i64,
    pub webhook_token: String,
}
