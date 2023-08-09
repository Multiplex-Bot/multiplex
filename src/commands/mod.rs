pub mod admin;
pub mod autocomplete;
pub mod delete;
pub mod edit;
pub mod export;
pub mod import;
pub mod info;
pub mod mate;
pub mod misc;
pub mod settings;

use anyhow::Error;
use mongodb::{options::FindOneAndUpdateOptions, Database};
use once_cell::sync::Lazy;
use s3::Bucket;

pub static UPSERT_OPTIONS: Lazy<Option<FindOneAndUpdateOptions>> = Lazy::new(|| {
    Some(
        FindOneAndUpdateOptions::builder()
            .upsert(Some(true))
            .build(),
    )
});

pub struct Data {
    pub database: Database,
    pub avatar_bucket: Bucket,
}
pub type CommandContext<'a> = poise::Context<'a, Data, Error>;
