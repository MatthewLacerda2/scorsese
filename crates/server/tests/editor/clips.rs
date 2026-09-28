//! A clip dragged onto another lane and a clip deleted: `clip_move` and
//! `clip_remove`, on the editor's allowlist and under its revision rule.

use scorsese_server::projects;
use serde_json::json;
use sqlx::postgres::PgPool;

use crate::{common, member, titled, tool};

#[sqlx::test]
async fn a_clip_moves_to_another_lane_and_is_deleted_by_hand(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let (id, revision) = titled(&pool, ana).await;

    let lane = json!({ "revision": revision, "arguments": { "kind": "video" } });
    let added = tool(address, &browser, id, "track_new", lane).await;
    assert_eq!(added.status, 200, "{}", added.body);
    let revision = revision + 1;

    let dragged = json!({ "revision": revision,
        "arguments": { "clip": "c1", "track": "v2", "start_seconds": 1.0 } });
    let moved = tool(address, &browser, id, "clip_move", dragged).await;
    assert_eq!(moved.status, 200, "{}", moved.body);
    let tracks = &moved.json()["project"]["document"]["tracks"];
    assert_eq!(tracks[0]["clips"], json!([]), "it left v1");
    assert_eq!(tracks[1]["clips"][0]["id"], "c1");
    assert_eq!(tracks[1]["clips"][0]["start"], 30);

    // Worked out on the timeline before the move: refused, nothing removed.
    let stale = json!({ "revision": revision, "arguments": { "clips": ["c1"] } });
    let refused = tool(address, &browser, id, "clip_remove", stale).await;
    assert_eq!(refused.status, 409, "{}", refused.body);

    let deleted = json!({ "revision": revision + 1, "arguments": { "clips": ["c1"] } });
    let removed = tool(address, &browser, id, "clip_remove", deleted).await;
    assert_eq!(removed.status, 200, "{}", removed.body);
    let stored = projects::open(&pool, ana, id).await.expect("opens");
    assert_eq!(stored.document.clips().count(), 0);
    assert_eq!(stored.document.assets.len(), 1, "the title is still there");
}
