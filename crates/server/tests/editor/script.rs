//! The Script button (#1445): `script_read` and `script_write` on the
//! editor's allowlist, the write under its revision rule, so a script the
//! assistant wrote meanwhile is never overwritten unseen.

use serde_json::json;
use sqlx::postgres::PgPool;

use crate::{common, member, titled, tool};

#[sqlx::test]
async fn the_script_is_read_and_rewritten_by_hand_on_the_revision_it_was_read_at(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, browser) = member(&pool, "ana@example.com").await;
    let (id, revision) = titled(&pool, ana).await;

    let none = tool(address, &browser, id, "script_read", json!({})).await;
    assert_eq!(none.status, 422, "no script yet: {}", none.body);

    let unpinned = json!({ "arguments": { "text": "# One" } });
    let refused = tool(address, &browser, id, "script_write", unpinned).await;
    assert_eq!(
        refused.status, 400,
        "a rewrite names its revision: {}",
        refused.body
    );

    let first = json!({ "revision": revision, "arguments": { "text": "# One" } });
    let written = tool(address, &browser, id, "script_write", first).await;
    assert_eq!(written.status, 200, "{}", written.body);
    let body = written.json();
    assert_eq!(body["project"]["revision"], revision + 1);
    assert_eq!(body["project"]["document"]["script"], "script.md");

    // Worked out on the script before: refused, the newer one stays.
    let stale = json!({ "revision": revision, "arguments": { "text": "# Stale" } });
    let moved = tool(address, &browser, id, "script_write", stale).await;
    assert_eq!(moved.status, 409, "{}", moved.body);

    let read = tool(address, &browser, id, "script_read", json!({})).await;
    assert_eq!(read.status, 200, "{}", read.body);
    let read = read.json();
    assert_eq!(read["said"][0]["text"], "# One");
    assert_eq!(
        read["project"]["revision"],
        revision + 1,
        "reading moves nothing"
    );
}
