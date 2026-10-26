//! A designed voice is its maker's alone (#687): every user's kept voices sit
//! in the one ElevenLabs account the server shares, so another user who learns
//! a `voice_id` can neither narrate with it nor look it up.

use std::net::SocketAddr;

use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, token};
use super::vendors::{Mock, serve, settled};
use super::{call, member};

/// What the studio mock names the voice kept out of its first candidate.
const KEPT: &str = "voice-of-gen-1";

/// A project with one line, sketched, read in `voice`.
fn read_in(voice: &str) -> serde_json::Value {
    json!({ "assets": [
        { "id": "vo", "kind": "generated_audio", "state": "sketch",
          "prompt": "Every city has a night editor.", "speech": { "voice_id": voice } }
    ] })
}

/// `user` designs a voice, pays for it and keeps its first candidate.
pub(super) async fn design_and_keep(
    address: SocketAddr,
    state: &AppState,
    user: UserId,
    who: &str,
) {
    let brief = json!({
        "prompt": "A warm, unhurried radio voice, for late-night stories.",
        "text": "Every city has a night editor, and every night editor has a story about \
                 the one that got away. This is that story, told the way she tells it."
    });
    let (quote, _) = call(address, who, "voice_design", brief).await;
    let (said, refused) = call(
        address,
        who,
        "voice_design",
        json!({ "confirm": token(&quote) }),
    )
    .await;
    assert!(!refused, "{said}");
    settled(state, user).await;
    let keep = json!({ "keep": "gen-1", "name": "Night editor" });
    let (said, refused) = call(address, who, "voice_design", keep).await;
    assert!(!refused, "{said}");
    settled(state, user).await;
}

#[sqlx::test]
async fn another_users_designed_voice_is_refused_at_generate(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    fund(&pool, bob, 10).await;
    design_and_keep(address, &state, ana, &hers).await;

    let theirs = super::stored(&pool, bob, read_in(KEPT)).await;
    let (said, refused) = call(address, &his, "generate", json!({ "project": theirs })).await;
    assert!(
        refused && said.contains("made by another account"),
        "{said}"
    );
    assert!(!said.contains("confirm: \""), "no token for it: {said}");
    let quotes: i64 = sqlx::query_scalar("SELECT count(*) FROM quotes")
        .fetch_one(&pool)
        .await
        .expect("the quotes count");
    assert_eq!(quotes, 0, "nothing was quoted");

    // The voice's maker narrates with it as with any other.
    let hers_too = super::stored(&pool, ana, read_in(KEPT)).await;
    let (quote, refused) = call(address, &hers, "generate", json!({ "project": hers_too })).await;
    assert!(!refused && quote.contains("confirm: \""), "{quote}");
}

#[sqlx::test]
async fn another_users_designed_voice_cannot_be_looked_up(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    design_and_keep(address, &state, ana, &hers).await;

    let project = super::stored(&pool, bob, json!({})).await;
    let asked = json!({ "project": project, "check": format!("  {KEPT} ") });
    let (said, refused) = call(address, &his, "voices", asked).await;
    assert!(
        refused && said.contains("made by another account"),
        "{said}"
    );
}
