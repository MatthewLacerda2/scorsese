//! A kind capped per user (`Registry::per_user`) runs no more than that many
//! of one user's at once, while other users' are claimed beside them — the
//! way one account's batched stills never hold the whole pool.

use scorsese_server::events::Events;
use scorsese_server::jobs::{Job, Kind, Outcome, Queue, Registry, State};
use serde_json::json;
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{account, enqueue, members, start_worker, stop_worker, when};

/// A kind with room for everybody's jobs, and a cap of two per user.
const POOLED: Kind = Kind {
    name: "pooled",
    limit: 8,
    budget: None,
};

#[sqlx::test]
async fn one_user_runs_no_more_than_their_share(pool: PgPool) {
    let (ana, bia) = (
        account(&pool, "ana@example.com").await,
        account(&pool, "bia@example.com").await,
    );
    let members = members(&pool).await;
    // Each job runs until its id is let go.
    let (release, released) = watch::channel(Vec::<i64>::new());
    let registry =
        Registry::new()
            .per_user(POOLED, 2)
            .register(POOLED, move |job: Job, _context| {
                let mut released = released.clone();
                async move {
                    let _ = released.wait_for(|ids| ids.contains(&job.id)).await;
                    Outcome::Done(json!(null))
                }
            });
    let queue = Queue::new(Events::new());
    let mut anas = Vec::new();
    for _ in 0..3 {
        anas.push(enqueue(&members, ana, POOLED, json!({})).await);
    }
    let bias = enqueue(&members, bia, POOLED, json!({})).await;

    let worker = start_worker(&members, registry, &queue);
    when(&members, ana, anas[0].id, State::Running).await;
    when(&members, ana, anas[1].id, State::Running).await;
    when(&members, bia, bias.id, State::Running).await;
    // Long enough for the worker to have claimed the third, had it room.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let third = when(&members, ana, anas[2].id, State::Waiting).await;
    assert_eq!(third.state, State::Waiting, "ana has two running already");

    // One of hers ends, and her third is taken.
    release.send_modify(|ids| ids.push(anas[0].id));
    when(&members, ana, anas[2].id, State::Running).await;
    release.send_modify(|ids| ids.extend([anas[1].id, anas[2].id, bias.id]));
    when(&members, ana, anas[2].id, State::Done).await;
    stop_worker(worker).await;
}
