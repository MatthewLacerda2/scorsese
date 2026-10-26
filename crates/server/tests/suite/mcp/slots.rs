//! A kept voice takes one of the shared ElevenLabs account's voice slots
//! (#1261): a user keeps at most their share, nobody keeps one once the
//! account is full, and either is said in plain words before the vendor is
//! asked.

use std::net::SocketAddr;

use scorsese_server::db::UserId;
use scorsese_server::designs::owners::KEPT_PER_USER;
use scorsese_server::http::AppState;
use scorsese_server::jobs::elevenlabs::ElevenLabs;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::fund;
use super::vendors::{Mock, serve_on, settled};
use super::voicing::design_and_keep;
use super::{call, member};

/// `user` keeps candidate `chosen` of the design they already have; why the
/// keep job failed, if it did.
async fn keep(
    pool: &PgPool,
    (address, state): (SocketAddr, &AppState),
    (user, who): (UserId, &str),
    chosen: &str,
) -> Option<String> {
    let asked = json!({ "keep": chosen, "name": "Another" });
    let (said, refused) = call(address, who, "voice_design", asked).await;
    assert!(!refused, "{said}");
    settled(state, user).await;
    sqlx::query_scalar(
        "SELECT error FROM jobs WHERE kind = 'voice_keep' AND user_id = $1
         ORDER BY id DESC LIMIT 1",
    )
    .bind(user.get())
    .fetch_one(pool)
    .await
    .expect("the keep job")
}

#[sqlx::test]
async fn a_user_keeps_no_more_than_their_share(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve_on(&pool, &mock, ElevenLabs::default()).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    design_and_keep(address, &state, ana, &hers).await;
    let ana = (ana, hers.as_str());
    for chosen in ["gen-2", "gen-3"] {
        assert_eq!(keep(&pool, (address, &state), ana, chosen).await, None);
    }
    assert_eq!(mock.kept(), 3);
    assert_eq!(KEPT_PER_USER, 3);

    let why = keep(&pool, (address, &state), ana, "gen-1").await;
    let why = why.expect("a fourth voice is refused");
    assert!(why.contains("already keep 3"), "{why}");
    assert!(why.contains("Voice Library"), "{why}");
    assert_eq!(mock.kept(), 3, "the vendor was never asked");
}

#[sqlx::test]
async fn nobody_keeps_a_voice_once_the_account_is_full(pool: PgPool) {
    let mock = Mock::default();
    let plan = ElevenLabs {
        voice_slots: 2,
        ..ElevenLabs::default()
    };
    let (address, state, _worker) = serve_on(&pool, &mock, plan).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    fund(&pool, ana, 10).await;
    fund(&pool, bob, 10).await;
    design_and_keep(address, &state, ana, &hers).await;
    design_and_keep(address, &state, bob, &his).await;

    let why = keep(&pool, (address, &state), (bob, &his), "gen-2").await;
    let why = why.expect("a third voice on a two-slot account is refused");
    assert!(why.contains("all 2 of its slots"), "{why}");
    assert_eq!(mock.kept(), 2, "the vendor was never asked");
}
