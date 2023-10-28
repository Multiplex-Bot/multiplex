use anyhow::Result;
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use crate::{
    models::{DBMate, DBMate__new},
    utils::misc::envvar,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupperboxExport {
    pub tuppers: Vec<Tupper>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tupper {
    pub id: i64,
    pub user_id: String,
    pub name: String,
    pub position: i64,
    pub avatar_url: String,
    pub brackets: Vec<String>,
    pub posts: i64,
    pub show_brackets: bool,
    pub birthday: Option<String>,
    pub description: Option<String>,
    pub tag: Option<String>,
    pub group_id: Option<i64>,
    pub group_pos: Option<i64>,
    pub created_at: Option<String>,
    pub nick: Option<String>,
}

impl Tupper {
    pub fn to_mate(&self, user_id: UserId) -> Result<DBMate> {
        Ok(DBMate__new! {
            user_id = user_id.get() as i64,
            autoproxy = false,
            name = self.name.clone(),
            avatar = if !self.avatar_url.is_empty() {
                self.avatar_url.clone()
            } else {
                envvar("DEFAULT_AVATAR_URL")
            },
            is_public = true,
            bio = self.description.clone(),
            prefix = if !self.brackets[0].is_empty() {
                Some(self.brackets[0].clone())
            } else {
                None
            },
            postfix = if !self.brackets[1].is_empty() {
                Some(self.brackets[1].clone())
            } else {
                None
            },
            display_name = self.nick.clone(),
        })
    }
}
