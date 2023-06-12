use std::num::NonZeroU64;

use anyhow::{Context, Result};

use mongodb::Database;

use poise::serenity_prelude::{
    CacheHttp, ChannelId, Context as SerenityContext, CreateAttachment, ExecuteWebhook, Message,
    Webhook, WebhookId,
};

use crate::models::{DBChannel, DBCollective, DBMate, DBMessage};
