use anyhow::Result;
use mongodb::{
    bson::{self, doc},
    Collection,
};

use crate::models::DBGuild;

pub async fn update_guild_settings(
    collection: &Collection<DBGuild>,
    guild: DBGuild,
    proxy_logs_channel_id: Option<i64>,
) -> Result<()> {
    let mut new_guild = guild.clone();

    if proxy_logs_channel_id.is_some() {
        new_guild.proxy_logs_channel_id = proxy_logs_channel_id;
    }

    collection
        .update_one(
            doc! {
                "id": new_guild.id
            },
            doc! { "$set": bson::to_bson(&new_guild).unwrap() },
            None,
        )
        .await?;

    Ok(())
}
