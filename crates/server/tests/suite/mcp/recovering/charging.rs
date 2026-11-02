//! Stopped between keeping a line and charging for it: charged, once.

use std::time::Duration;

use sqlx::Executor;
use sqlx::postgres::PgPool;

use super::super::paying::{fund, narrated, quote_and_confirm};
use super::super::vendors::{Mock, serve_idle, settled, work};
use super::super::{member, stored};
use super::{FUNDED, balance};
use scorsese_server::jobs::elevenlabs::ElevenLabs;

/// The advisory lock a charge waits on while [`gate_charges`] holds it.
const GATE: i64 = 1645;

#[sqlx::test]
async fn a_line_stopped_between_keeping_and_charging_is_charged_once(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, files) = serve_idle(&pool).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;
    let mut gate = gate_charges(&pool).await;
    let first = work(&pool, &state, &files, &mock, ElevenLabs::default());
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    let waiting = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM pg_locks
             WHERE locktype = 'advisory' AND objid = $1 AND NOT granted",
        )
        .bind(GATE)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    for _ in 0..400 {
        if waiting().await > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(waiting().await > 0, "the charge never reached the gate");
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM library_items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(kept >= 1, "the line is kept before it is charged");

    // The server stops with the line kept and its charge not committed.
    first.send(true).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    gate.execute(sqlx::query("SELECT pg_advisory_unlock($1)").bind(GATE))
        .await
        .unwrap();
    let _second = work(&pool, &state, &files, &mock, ElevenLabs::default());

    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.spoken(), 1, "never sent twice");
    let charges: Vec<i64> =
        sqlx::query_scalar("SELECT amount_micros FROM credit_entries WHERE kind = 'charge'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(charges.len(), 1, "charged once: {charges:?}");
    assert_eq!(balance(&pool, ana).await, FUNDED + charges[0]);
}

/// Hold every charge back until the returned connection lets go of [`GATE`].
async fn gate_charges(pool: &PgPool) -> sqlx::PgConnection {
    pool.execute(
        "CREATE FUNCTION gate_charge() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN PERFORM pg_advisory_xact_lock_shared(1645); RETURN NEW; END $$",
    )
    .await
    .unwrap();
    pool.execute(
        "CREATE TRIGGER gate_charge BEFORE INSERT ON credit_entries
         FOR EACH ROW WHEN (NEW.kind = 'charge') EXECUTE FUNCTION gate_charge()",
    )
    .await
    .unwrap();
    let mut held = pool.acquire().await.unwrap().detach();
    held.execute(sqlx::query("SELECT pg_advisory_lock($1)").bind(GATE))
        .await
        .unwrap();
    held
}
