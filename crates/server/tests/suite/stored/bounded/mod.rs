//! A member's in-request tool calls are bounded (#1649): at most three run at
//! once, and none runs past its deadline.
//!
//! The calls (`timing`, `dropped`) are `stock_search`es against a library that holds every search
//! until the test lets it go, so a call is "running" for exactly as long as
//! the test says, with no vendor and nothing big.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use scorsese_core::{Fps, Project};
use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, StockError};
use scorsese_server::accounts::users;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::projects;
use sqlx::postgres::PgPool;

use crate::common;

mod dropped;
mod timing;

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

/// A member with one project, tools searching `gate`'s library, and where
/// their projects are laid out to run tools on.
async fn held(pool: &PgPool, label: &str, gate: &Arc<Gate>) -> (AppState, UserId, i64, PathBuf) {
    let members = common::members(pool).await;
    let ana = users::create(pool, "ana@example.com")
        .await
        .expect("the member");
    let project = Project::new("p", Fps::default());
    let id = projects::create(&members, ana, &project)
        .await
        .expect("the project")
        .id;
    let files = common::files(label);
    let scratch = files
        .storage
        .scratch(ana)
        .parent()
        .expect("a scratch folder is in the member's scratch directory")
        .to_path_buf();
    let state = AppState::new(members, files).stocked_from(Arc::new(Held(gate.clone())));
    (state, ana, id, scratch)
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

/// Opens the gate when dropped.
struct Opens(Arc<Gate>);

impl Drop for Opens {
    fn drop(&mut self) {
        self.0.open();
    }
}

/// How many projects are laid out under `scratch` right now.
fn laid_out(scratch: &Path) -> usize {
    std::fs::read_dir(scratch).map_or(0, Iterator::count)
}
