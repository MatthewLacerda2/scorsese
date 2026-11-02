//! A member's in-request tool calls are bounded (#1649): at most three run at
//! once, and none runs past its deadline.
//!
//! The calls are `stock_search`es against a library that holds every search
//! until the test lets it go, so a call is "running" for exactly as long as
//! the test says, with no vendor and nothing big.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use scorsese_core::{Fps, Project};
use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, StockError};
use scorsese_server::accounts::users;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::projects;
use scorsese_server::tools::Client;
use serde_json::json;
use sqlx::postgres::PgPool;

use crate::common;

/// How many searches have reached the library, and whether they may answer.
#[derive(Default)]
struct Gate {
    state: Mutex<(usize, bool)>,
    moved: Condvar,
}

impl Gate {
    fn entered(&self) -> usize {
        self.state.lock().expect("the gate").0
    }

    fn open(&self) {
        self.state.lock().expect("the gate").1 = true;
        self.moved.notify_all();
    }
}

/// A library whose every search waits at the gate.
struct Held(Arc<Gate>);

impl Library for Held {
    fn name(&self) -> &'static str {
        "Held"
    }

    fn page(&self, _: &Query, _: u32) -> Result<Page, StockError> {
        let mut state = self.0.state.lock().expect("the gate");
        state.0 += 1;
        self.0.moved.notify_all();
        while !state.1 {
            state = self.0.moved.wait(state).expect("the gate");
        }
        Ok(Page {
            candidates: Vec::new(),
            total: 0,
        })
    }

    fn one(&self, _: Medium, _: u64) -> Result<Option<Candidate>, StockError> {
        Ok(None)
    }

    fn hosts(&self) -> &'static [&'static str] {
        &["example.com"]
    }

    fn download(&self, _: &str, _: u64, _: &mut dyn std::io::Write) -> Result<u64, StockError> {
        Err(StockError::Provider {
            library: "Held",
            said: "nothing is downloaded here".to_owned(),
        })
    }
}

/// A member with one project, and tools searching `gate`'s library.
async fn held(pool: &PgPool, label: &str, gate: &Arc<Gate>) -> (AppState, UserId, i64) {
    let members = common::members(pool).await;
    let ana = users::create(pool, "ana@example.com")
        .await
        .expect("the member");
    let project = Project::new("p", Fps::default());
    let id = projects::create(&members, ana, &project)
        .await
        .expect("the project")
        .id;
    let state =
        AppState::new(members, common::files(label)).stocked_from(Arc::new(Held(gate.clone())));
    (state, ana, id)
}

/// Waits until `gate` has seen `n` searches arrive, for a few seconds.
async fn arrived(gate: &Gate, n: usize) {
    let asked = Instant::now();
    while gate.entered() < n {
        assert!(
            asked.elapsed() < Duration::from_secs(20),
            "{} of {n} searches arrived",
            gate.entered()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[sqlx::test]
async fn a_fourth_call_at_once_waits_for_one_of_the_three(pool: PgPool) {
    let gate = Arc::new(Gate::default());
    let (state, ana, id) = held(&pool, "bounded-at-once", &gate).await;
    // A query each: an answer the stock cache holds never reaches the library.
    let calls: Vec<_> = (0..4)
        .map(|n| {
            let state = state.clone();
            let search = json!({ "project": id, "query": format!("sea {n}") });
            tokio::spawn(async move {
                let client = Client::External { token: None };
                state.tools.call(ana, client, "stock_search", &search).await
            })
        })
        .collect();

    arrived(&gate, 3).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(gate.entered(), 3, "the fourth waits for a permit");

    gate.open();
    for call in calls {
        let answered = call.await.expect("the call ends");
        assert!(answered.is_ok(), "{:?}", answered.err());
    }
    assert_eq!(gate.entered(), 4, "and runs once one is free");
}

#[sqlx::test]
async fn a_call_past_its_deadline_ends_saying_so(pool: PgPool) {
    let gate = Arc::new(Gate::default());
    let (mut state, ana, id) = held(&pool, "bounded-deadline", &gate).await;
    state.tools = state.tools.clone().bounded(3, Duration::from_millis(300));
    let search = json!({ "project": id, "query": "sea" });
    let client = Client::External { token: None };

    let asked = Instant::now();
    let answered = state.tools.call(ana, client, "stock_search", &search).await;
    let said = answered.err().expect("refused past its deadline");
    assert!(said.contains("stopped"), "{said}");
    assert!(
        asked.elapsed() < Duration::from_secs(10),
        "answered at the deadline"
    );
    gate.open();
}
