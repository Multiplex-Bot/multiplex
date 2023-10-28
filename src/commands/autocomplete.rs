use mongodb::bson::doc;
use strsim::normalized_damerau_levenshtein;

use super::CommandContext;
use crate::models::DBMate;

pub async fn mate(ctx: CommandContext<'_>, current_arg: &str) -> Vec<String> {
    let database = &ctx.data().database;
    let mates_collection = database.collection::<DBMate>("mates");

    let mut mates: Vec<String> = mates_collection
        .distinct(
            "name",
            doc! { "user_id": ctx.author().id.get() as i64 },
            None,
        )
        .await
        .expect("Failed to get all mates!")
        .iter()
        .map(|bson| bson.as_str().unwrap().to_string())
        .collect();

    mates.sort_by(|a, b| {
        normalized_damerau_levenshtein(b, current_arg)
            .partial_cmp(&normalized_damerau_levenshtein(a, current_arg))
            .unwrap()
    });
    mates.shrink_to(25);

    mates
}
