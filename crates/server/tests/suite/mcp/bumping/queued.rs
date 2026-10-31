//! A shot queued and paid for by the build before this one, its job left for
//! this build's worker.

use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::jobs::elevenlabs::ElevenLabs;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::super::paying::{fund, quote_and_confirm};
use super::super::vendors::{Mock, serve_idle, work};
use super::super::{member, stored};
use super::{RENAMED, WRITTEN, balance};

/// The ticket a shot Veo had taken holds — the one the mock answers for.
pub(super) const TICKET: &str = "operations/mock-1";

/// What the job's payload carries.
pub(super) enum Carried {
    /// The document as it was quoted, at this build's version.
    Current,
    /// The document as a build at this version wrote it, naming the asset
    /// [`WRITTEN`].
    At(u64),
}

/// A shot queued and paid for by a build before this one.
pub(super) struct Before {
    pub(super) state: AppState,
    pub(super) user: UserId,
    pub(super) project: i64,
    /// The balance once the shot was reserved.
    pub(super) reserved: i64,
    _worker: watch::Sender<bool>,
}

/// Ana's shot, quoted and confirmed while no worker runs; its job's payload
/// made into what an older build would have queued, holding `ticket` when
/// Veo had taken the shot before the deploy; and then a worker of this build
/// started, as on the new build's first start.
pub(super) async fn enqueued_before_the_bump(
    pool: &PgPool,
    mock: &Mock,
    carried: Carried,
    ticket: Option<&str>,
) -> Before {
    let (address, state, files) = serve_idle(pool).await;
    let (ana, who) = member(pool, "ana@example.com").await;
    let shot = json!({ "assets": [
        { "id": RENAMED, "kind": "generated_video", "state": "sketch",
          "prompt": "a lighthouse at dawn" }
    ] });
    let project = stored(pool, ana, shot).await;
    fund(pool, ana, 10).await;
    let (said, refused) = quote_and_confirm(address, &who, project).await;
    assert!(!refused, "{said}");

    let mut payload: Value = sqlx::query_scalar("SELECT payload FROM jobs WHERE kind = 'veo_shot'")
        .fetch_one(pool)
        .await
        .expect("the queued job");
    if let Carried::At(version) = carried {
        payload["asset"] = json!(WRITTEN);
        payload["document"]["schema_version"] = json!(version);
        payload["document"]["assets"][0]["id"] = json!(WRITTEN);
    }
    sqlx::query(
        "UPDATE jobs SET payload = $1, ticket = $2,
                ticket_at = CASE WHEN $2::text IS NULL THEN NULL ELSE now() END
         WHERE kind = 'veo_shot'",
    )
    .bind(&payload)
    .bind(ticket)
    .execute(pool)
    .await
    .expect("the job is rewritten");
    sqlx::query("UPDATE veo_generations SET ticket = $1")
        .bind(ticket)
        .execute(pool)
        .await
        .expect("the audit row is rewritten");

    let reserved = balance(pool, ana).await;
    assert!(reserved < 10_000_000, "reserved: {reserved}");
    let worker = work(pool, &state, &files, mock, ElevenLabs::default());
    Before {
        state,
        user: ana,
        project,
        reserved,
        _worker: worker,
    }
}
