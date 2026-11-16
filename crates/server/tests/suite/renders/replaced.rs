//! A piece kept replaces the pieces of older revisions at its shape, over
//! any range, and keeps its own revision's, so seeking back stays free
//! (#1448): an edit-and-watch cycle never piles pieces against the quota.

use sqlx::postgres::PgPool;

use super::pieces::piece;
use super::previews::worker;
use super::{card, common, member, stored};
use scorsese_server::projects;
use serde_json::json;

/// The ranges of `id`'s renders, in order.
async fn ranges(pool: &PgPool, id: i64) -> Vec<String> {
    sqlx::query_scalar("SELECT settings ->> 'range' FROM renders WHERE project_id = $1 ORDER BY 1")
        .bind(id)
        .fetch_all(pool)
        .await
        .expect("the renders read")
}

#[sqlx::test]
async fn a_piece_of_a_newer_revision_replaces_the_older_ones_whatever_their_range(pool: PgPool) {
    let files = common::files("preview-pieces-replaced");
    let _worker = worker(&pool, &files).await;
    let (address, _) = common::serve_with(pool.clone(), files.clone()).await;
    let (ana, cookie) = member(&pool, "ana@example.com").await;
    let who = (address, cookie.as_str(), ana);
    let id = stored(&pool, ana, &card(|_| {})).await;
    piece(&pool, &files, who, id, "0:3").await;
    piece(&pool, &files, who, id, "3:").await;
    // Of the same revision, each range is kept beside the others.
    assert_eq!(ranges(&pool, id).await, ["0:3", "3:"]);

    let edited = card(|document| document["assets"][0]["color"] = json!("#112233"));
    projects::save(&pool, ana, id, 1, &edited)
        .await
        .expect("the edit is saved");
    piece(&pool, &files, who, id, "0:3").await;
    // The older revision's pieces are gone, the one not asked again included.
    assert_eq!(ranges(&pool, id).await, ["0:3"]);
}
