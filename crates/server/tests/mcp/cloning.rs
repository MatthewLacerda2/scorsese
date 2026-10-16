//! `voice_clone` on the web (#1263): consent first, only the user's own
//! uploaded recordings, a voice slot taken like a kept design's, and
//! forgotten by the same job.

use std::net::SocketAddr;

use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::vendors::{Mock, serve, settled};
use super::{call, member};

/// An uploaded recording in `user`'s library, imported into a new project of
/// theirs: the project, and the asset id.
async fn recorded(
    pool: &PgPool,
    (address, state): (SocketAddr, &AppState),
    (user, who): (UserId, &str),
) -> (i64, String) {
    let sha = scorsese_core::hash_bytes(b"ID3 a recording");
    let item: i64 = sqlx::query_scalar(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'me reading.mp3', 'audio', 'mp3', 15, '{}') RETURNING id",
    )
    .bind(user.get())
    .bind(&sha)
    .fetch_one(pool)
    .await
    .expect("the upload's row");
    let file = state.library.storage().library_file(user, &sha, "mp3");
    std::fs::create_dir_all(file.parent().expect("a library folder")).expect("the folder");
    std::fs::write(&file, b"ID3 a recording").expect("the recording");
    let project = super::stored(pool, user, json!({})).await;
    let bring = json!({ "project": project, "items": [item] });
    let (said, refused) = call(address, who, "import", bring).await;
    assert!(!refused, "{said}");
    let asset = super::document(pool, user, project).await.document.assets[0]
        .id
        .as_str()
        .to_owned();
    (project, asset)
}

#[sqlx::test]
async fn a_clone_needs_consent_and_takes_a_slot_until_forgotten(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (project, asset) = recorded(&pool, (address, &state), (ana, &hers)).await;
    let asked = json!({ "project": project, "assets": [asset], "name": "Ana" });

    let (said, refused) = call(address, &hers, "voice_clone", asked.clone()).await;
    assert!(refused && said.contains("your own voice"), "{said}");

    let mut consented = asked;
    consented["consent"] = json!(true);
    let (said, refused) = call(address, &hers, "voice_clone", consented).await;
    assert!(!refused, "{said}");
    settled(&state, ana).await;
    assert_eq!(mock.cloned(), 1);
    let (listed, _) = call(address, &hers, "voice_clone", json!({ "list": true })).await;
    assert!(
        listed.contains("clone-1") && listed.contains("me reading.mp3"),
        "{listed}"
    );
    let slots = scorsese_server::designs::owners::slots(&pool, ana)
        .await
        .expect("the slots");
    assert_eq!(slots.mine, 1, "a clone takes a voice slot");

    let (said, refused) = call(
        address,
        &hers,
        "voice_clone",
        json!({ "forget": "clone-1" }),
    )
    .await;
    assert!(!refused, "{said}");
    settled(&state, ana).await;
    assert_eq!(mock.forgotten(), 1);
    let slots = scorsese_server::designs::owners::slots(&pool, ana)
        .await
        .expect("the slots");
    assert_eq!(slots.mine, 0, "forgetting it frees the slot");
}

#[sqlx::test]
async fn only_ones_own_uploaded_recording_is_cloned_from(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (_, his) = member(&pool, "bob@example.com").await;
    let (project, asset) = recorded(&pool, (address, &state), (ana, &hers)).await;
    let asked = json!({ "project": project, "assets": [asset], "name": "Ana", "consent": true });

    let (said, refused) = call(address, &his, "voice_clone", asked).await;
    assert!(refused, "another user's project is not theirs: {said}");
    let missing = json!({ "project": project, "assets": ["nobody"], "name": "A", "consent": true });
    let (said, refused) = call(address, &hers, "voice_clone", missing).await;
    assert!(refused && said.contains("no asset nobody"), "{said}");
    assert_eq!(mock.cloned(), 0, "the vendor was never asked");
}
