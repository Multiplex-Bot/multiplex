pub mod mate;
pub mod stats;

use anyhow::Error;
use mongodb::{options::FindOneAndUpdateOptions, Database};
use once_cell::sync::Lazy;

pub static UPSERT_OPTIONS: Lazy<Option<FindOneAndUpdateOptions>> = Lazy::new(|| {
    Some(
        FindOneAndUpdateOptions::builder()
            .upsert(Some(true))
            .build(),
    )
});

pub struct Data {
    pub database: Database,
}
pub type CommandContext<'a> = poise::Context<'a, Data, Error>;
