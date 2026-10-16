//! Kinds that share a [`Budget`] run no more at once, together, than its
//! size (#1261) — the way every ElevenLabs kind shares the plan's concurrent
//! requests.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use scorsese_server::events::Events;
use scorsese_server::jobs::{Budget, Kind, Outcome, Queue, Registry, State};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{account, enqueue, members, start_worker, stop_worker, when};

const VENDOR: Budget = Budget { name: "vendor" };

/// Two kinds that would each run three at once, but share [`VENDOR`].
const SPEAK: Kind = Kind {
    name: "speak",
    limit: 3,
    budget: Some(VENDOR),
};
const DESIGN: Kind = Kind {
    name: "design",
    limit: 3,
    budget: Some(VENDOR),
};

#[sqlx::test]
async fn kinds_sharing_a_budget_run_no_more_than_it_together(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let members = members(&pool).await;
    let (now, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let counted = || {
        let (now, most) = (Arc::clone(&now), Arc::clone(&most));
        move |_job, _context| {
            let (now, most) = (Arc::clone(&now), Arc::clone(&most));
            async move {
                most.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(150)).await;
                now.fetch_sub(1, Ordering::SeqCst);
                Outcome::Done(json!(null))
            }
        }
    };
    let registry = Registry::new()
        .budget(VENDOR, 2)
        .register(SPEAK, counted())
        .register(DESIGN, counted());
    let queue = Queue::new(Events::new());
    let mut jobs = Vec::new();
    for kind in [SPEAK, DESIGN, SPEAK, DESIGN, SPEAK, DESIGN] {
        jobs.push(enqueue(&members, ana, kind, json!({})).await);
    }

    let worker = start_worker(&members, registry, &queue);
    for job in &jobs {
        when(&members, ana, job.id, State::Done).await;
    }
    stop_worker(worker).await;
    assert_eq!(most.load(Ordering::SeqCst), 2);
}
