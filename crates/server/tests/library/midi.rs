//! MIDI is not a library kind (#964): with `synth_import` and `synth_export`
//! off the tool list (#785) nothing on the web could use a `.mid`, so one is
//! refused when it is announced, like any other file scorsese cannot use, and
//! the database holds no row of the kind.

use sqlx::postgres::PgPool;

use crate::common::{self, TUNE, request};
use crate::{announce, member};

#[sqlx::test]
async fn a_midi_file_is_refused_before_a_byte_is_sent(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("upload-midi")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    for name in ["Rag.MID", "rag.midi"] {
        let refused = announce(address, &ana, name, TUNE).await;
        assert_eq!(refused.status, 415, "{name}: {}", refused.body);
        assert!(refused.body.contains("cannot use"), "{}", refused.body);
    }
    let listed = request(address, "GET", "/api/library", &[&ana], None).await;
    assert_eq!(listed.json(), serde_json::json!([]));
}

#[sqlx::test]
async fn no_row_can_hold_the_retired_kind(pool: PgPool) {
    let (user, _) = member(&pool, "ana@example.com").await;
    let stored = sqlx::query(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'Rag.mid', 'midi', 'mid', 42, '{}')",
    )
    .bind(user.get())
    .bind("c".repeat(64))
    .execute(&pool)
    .await;
    let refused = stored.expect_err("the kind is gone from the constraint");
    assert!(
        refused.to_string().contains("library_items_kind_check"),
        "{refused}"
    );
}
