//! Corrections to a transcript made at once (#1549): each is applied to what
//! the last one left, so none is lost.

use futures_util::future::join_all;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::stopped::imported;
use crate::vendors::{Mock, serve};
use crate::{call, member};

/// The words `one` … `eight`, a second apart.
fn eight() -> Value {
    let words: Vec<Value> = [
        "one", "two", "three", "four", "five", "six", "seven", "eight",
    ]
    .iter()
    .enumerate()
    .map(|(at, text)| json!({ "text": text, "start": at as f64, "end": at as f64 + 0.5 }))
    .collect();
    json!({ "words": words })
}

#[sqlx::test]
async fn fixes_made_at_once_are_all_kept(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, item) = imported(&pool, &state, ana, address, &who).await;
    sqlx::query("UPDATE library_items SET words = $2 WHERE id = $1")
        .bind(item)
        .bind(eight())
        .execute(&pool)
        .await
        .expect("transcribed");

    let fixes = (1..=8).map(|word| {
        let asked =
            json!({ "project": id, "asset": asset, "fixes": [{ "word": word, "text": "fixed" }] });
        let who = who.clone();
        async move { call(address, &who, "transcript_fix", asked).await }
    });
    for (said, refused) in join_all(fixes).await {
        assert!(!refused, "{said}");
    }
    let kept: Value = sqlx::query_scalar("SELECT words FROM library_items WHERE id = $1")
        .bind(item)
        .fetch_one(&pool)
        .await
        .expect("the row");
    let texts: Vec<&str> = kept["words"]
        .as_array()
        .expect("words")
        .iter()
        .map(|word| word["text"].as_str().expect("text"))
        .collect();
    assert_eq!(texts, ["fixed"; 8], "{kept}");
}
