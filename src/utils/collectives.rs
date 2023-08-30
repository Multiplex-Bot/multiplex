use anyhow::{Context, Result};
use chrono::Utc;
use mongodb::{
    bson::{self, doc, oid::ObjectId},
    Collection,
};
use poise::serenity_prelude::UserId;

use crate::models::{DBCollective, DBCollective__new, SwitchLog};

pub async fn get_or_create_collective(
    collection: &Collection<DBCollective>,
    user_id: UserId,
) -> Result<DBCollective> {
    let collective = collection
        .find_one(doc! { "user_id": user_id.get() as i64 }, None)
        .await?;

    if let Some(collective) = collective {
        Ok(collective)
    } else {
        let new_collective = DBCollective__new! {
            user_id = user_id.get() as i64,
            is_public = true,
        };

        collection
            .insert_one(new_collective.clone(), None)
            .await
            .context("Failed to create new collective in database; try again later!")?;

        Ok(new_collective)
    }
}

pub async fn update_switch_logs(
    collection: &Collection<DBCollective>,
    collective: &DBCollective,
    mate_id: Option<ObjectId>,
    previous_mate_id: Option<ObjectId>,
) -> Result<()> {
    let mut switch_logs = collective.switch_logs.clone().unwrap_or_default();

    switch_logs.insert(
        0,
        SwitchLog {
            date: Utc::now(),
            mate_id,
            previous_mate_id,
            unswitch: mate_id.is_none(),
        },
    );

    switch_logs.truncate(5);

    collection
        .update_one(
            doc! { "user_id": collective.user_id},
            doc! {
                "$set": { "switch_logs": bson::to_bson(&switch_logs).unwrap() }
            },
            None,
        )
        .await?;

    Ok(())
}
