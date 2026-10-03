//! Editing a stored project through the registry's own tools: the edit lands
//! in the row, for its owner only, and every call is on record.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, document, member};

#[sqlx::test]
async fn a_project_is_made_found_and_edited_by_its_id(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;

    let (made, refused) = call(address, &token, "project_new", json!({ "name": "Teaser" })).await;
    assert!(!refused, "{made}");
    let id: i64 = made
        .split("its id is ")
        .nth(1)
        .and_then(|rest| rest.split('.').next())
        .and_then(|id| id.parse().ok())
        .unwrap_or_else(|| panic!("no id in {made}"));
    let (listed, _) = call(address, &token, "project_list", json!({})).await;
    assert!(
        listed.starts_with(&format!("{id} — Teaser (revision ")),
        "{listed}"
    );

    let before = document(&pool, ana, id).await.summary.revision;
    let track = json!({ "project": id, "kind": "video", "track": "v1" });
    let (said, refused) = call(address, &token, "track_new", track).await;
    assert!(!refused, "{said}");
    let after = document(&pool, ana, id).await;
    assert_eq!(after.summary.revision, before + 1);
    assert_eq!(after.document.tracks.len(), 1, "the edit is in the row");

    let (read, refused) = call(address, &token, "project_read", json!({ "project": id })).await;
    assert!(!refused && read.contains("\"v1\""), "{read}");
    assert!(
        !read.contains("scratch"),
        "the folder never reaches a client: {read}"
    );
    // A tool that changes nothing writes nothing.
    assert_eq!(document(&pool, ana, id).await.summary.revision, before + 1);
}

#[sqlx::test]
async fn another_users_project_is_not_there(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    let hers = super::stored(&pool, ana, json!({})).await;
    for (tool, arguments) in [
        ("project_read", json!({ "project": hers })),
        (
            "track_new",
            json!({ "project": hers, "kind": "video", "track": "v9" }),
        ),
        ("generate", json!({ "project": hers })),
        ("render", json!({ "project": hers })),
    ] {
        let (said, refused) = call(address, &bob, tool, arguments).await;
        assert!(refused, "{tool}: {said}");
        assert!(said.contains("no such project"), "{tool}: {said}");
    }
    assert!(document(&pool, ana, hers).await.document.tracks.is_empty());
    let (none, _) = call(address, &bob, "project_list", json!({})).await;
    assert!(none.contains("no projects"), "{none}");
}

/// On a machine everybody shares, a file argument is a path inside the
/// project, and nothing is written to the server's disk for anyone to keep.
#[sqlx::test]
async fn a_path_outside_the_project_is_refused(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, json!({})).await;
    for file in ["/etc/passwd", "../../other/assets/x.mp4"] {
        let arguments = json!({ "project": id, "file": file });
        let (said, refused) = call(address, &token, "look", arguments).await;
        assert!(
            refused && said.contains("not a path inside the project"),
            "{said}"
        );
    }
    let still = json!({ "project": id, "at": 0, "out": "/tmp/frame.png" });
    let (said, refused) = call(address, &token, "still", still).await;
    assert!(refused && said.contains("not taken"), "{said}");
    let withheld = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "synth_export", "arguments": { "project": id } }
    });
    let reply = super::post(address, &token, &withheld).await.json();
    assert_eq!(reply["error"]["code"], json!(-32601), "{reply}");
}

#[sqlx::test]
async fn every_call_is_on_record_as_the_clients(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, token) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, json!({})).await;
    call(address, &token, "project_read", json!({ "project": id })).await;
    call(
        address,
        &token,
        "look",
        json!({ "project": id, "file": "/x" }),
    )
    .await;
    let rows: Vec<(String, String, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT client, tool, project_id, outcome FROM tool_calls WHERE user_id = $1 ORDER BY id",
    )
    .bind(ana.get())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (
                "external".into(),
                "project_read".into(),
                Some(id),
                Some("answered".into())
            ),
            (
                "external".into(),
                "look".into(),
                Some(id),
                Some("refused".into())
            ),
        ]
    );
}
