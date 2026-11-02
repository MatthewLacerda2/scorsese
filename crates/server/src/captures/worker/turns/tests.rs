//! [`super::Turns`] (#1634): one page a turn, turns by owner.
//!
//! The capture is a stand-in that only writes down which page it was asked
//! for: what is tested is the order, not the browser.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use super::super::{Isolation, Worker};
use super::Turns;
use crate::captures::{PROJECT, Spool};
use crate::scratch;

/// A worker whose every capture appends `<project> <index>` to `log`.
fn logging_worker(spool: &Spool, root: &Path) -> (Worker, PathBuf) {
    let log = root.join("captured.log");
    let program = root.join("capture-one");
    let script = format!("#!/bin/sh\necho \"$3 $7\" >> '{}'\n", log.display());
    std::fs::write(&program, script).expect("the stand-in");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("runnable");
    let worker = Worker {
        spool: spool.clone(),
        isolation: Isolation::Process {
            program,
            sandbox: true,
            library: None,
        },
        deadline: |_| Duration::from_secs(30),
    };
    (worker, log)
}

/// Job `job` of `user`, asking for `pages` pages: its project's `cache/` a
/// link to the user's page cache, as the server lays one out.
fn asked(spool: &Spool, job: i64, user: u64, pages: usize) {
    let folder = spool.job(job);
    let project = folder.join(PROJECT);
    std::fs::create_dir_all(&project).expect("the project");
    let cache = spool.root().join(format!("pages/{user}/{job}"));
    std::fs::create_dir_all(&cache).expect("the page cache");
    let cache = std::fs::canonicalize(cache).expect("canonical");
    std::os::unix::fs::symlink(cache, project.join("cache")).expect("cache/");
    let one = json!({ "page": "pages/p.html", "width": 64, "height": 36,
                      "fps": { "num": 10, "den": 1 }, "duration": 1.0 });
    let ask = json!({ "requests": vec![one; pages] });
    std::fs::write(folder.join("ask.json"), ask.to_string()).expect("ask");
    // Asked in order, a moment apart.
    std::thread::sleep(Duration::from_millis(20));
}

/// Which job each capture was for, in order.
fn served(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let project = line.split(' ').next().unwrap_or_default();
            let job = Path::new(project).parent().and_then(Path::file_name);
            job.map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn two_owners_jobs_are_served_a_page_each_in_turn() {
    let root = scratch::dir("captures-turns");
    let spool = Spool::new(root.join("spool"));
    let (worker, log) = logging_worker(&spool, &root);
    asked(&spool, 1, 7, 5);
    asked(&spool, 2, 7, 1);
    asked(&spool, 3, 8, 1);

    let mut turns = Turns::default();
    while worker.take_turn(&mut turns) {}

    assert_eq!(
        served(&log),
        [
            "job-1", "job-3", "job-1", "job-1", "job-1", "job-1", "job-2"
        ],
        "owner 8's page waits behind one of owner 7's, not five"
    );
    for job in [1, 2, 3] {
        let answer = std::fs::read(spool.job(job).join("answer.json")).expect("answered");
        let answer: serde_json::Value = serde_json::from_slice(&answer).expect("JSON");
        let failed = answer["failed"].as_array().expect("one per page");
        assert!(failed.iter().all(serde_json::Value::is_null), "{answer}");
    }
}

#[test]
fn a_job_withdrawn_part_way_is_dropped_and_never_answered() {
    let root = scratch::dir("captures-withdrawn");
    let spool = Spool::new(root.join("spool"));
    let (worker, log) = logging_worker(&spool, &root);
    asked(&spool, 1, 7, 3);
    let mut turns = Turns::default();
    assert!(worker.take_turn(&mut turns));
    std::fs::remove_dir_all(spool.job(1)).expect("withdrawn");
    assert!(!worker.take_turn(&mut turns), "nothing left to capture");
    assert_eq!(served(&log), ["job-1"]);
}
