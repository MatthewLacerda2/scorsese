//! A long page is a part a turn (#1708): no turn of the launcher's one
//! capture slot draws more than `captures::TURN` frames, so another account's
//! page goes between the parts of a long one.
//!
//! The capture is a stand-in that only writes down what it was asked for:
//! what is tested is the order and the parts, not the browser. That the parts
//! join into the frames one capture draws is the render crate's
//! `same_frames`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use scorsese_server::captures::worker::{Isolation, Turns, Worker};
use scorsese_server::captures::{PROJECT, Spool};

use crate::common::scratch;

/// A worker whose every capture appends its arguments to `log`, a line each.
fn logging_worker(spool: &Spool, root: &Path) -> (Worker, PathBuf) {
    let log = root.join("captured.log");
    let program = root.join("capture-one");
    let script = format!("#!/bin/sh\necho \"$*\" >> '{}'\n", log.display());
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

/// Job `job` of `user`, asking for one page `seconds` long at 10 fps: its
/// project's `cache/` a link to the user's page cache, as the server lays one
/// out.
fn asked(spool: &Spool, job: i64, user: u64, seconds: f64) {
    let folder = spool.job(job);
    let project = folder.join(PROJECT);
    std::fs::create_dir_all(&project).expect("the project");
    let cache = spool.root().join(format!("pages/{user}/{job}"));
    std::fs::create_dir_all(&cache).expect("the page cache");
    let cache = std::fs::canonicalize(cache).expect("canonical");
    std::os::unix::fs::symlink(cache, project.join("cache")).expect("cache/");
    let page = json!({ "page": "pages/p.html", "width": 64, "height": 36,
                       "fps": { "num": 10, "den": 1 }, "duration": seconds });
    let ask = json!({ "requests": [page] });
    std::fs::write(folder.join("ask.json"), ask.to_string()).expect("ask");
    // Asked in order, a moment apart.
    std::thread::sleep(Duration::from_millis(20));
}

/// Each capture's job and the frames it was asked to draw, in order: `job-1
/// 0..1800`, or `job-1 all` for one asked for every frame at once.
fn served(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            let after = |flag: &str| {
                let at = words.iter().position(|word| *word == flag)?;
                words.get(at + 1).copied()
            };
            let project = Path::new(after("--project").unwrap_or_default());
            let job = project.parent().and_then(Path::file_name);
            let job = job.map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            format!("{job} {}", after("--frames").unwrap_or("all"))
        })
        .collect()
}

#[test]
fn a_long_page_is_captured_a_turn_of_frames_at_a_time() {
    let root = scratch::fresh("scorsese-captures-turns", "");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch folder");
    let spool = Spool::new(root.join("spool"));
    let (worker, log) = logging_worker(&spool, &root);
    // 2,000 frames, and a second of somebody else's page asked just after.
    asked(&spool, 1, 7, 199.9);
    asked(&spool, 2, 8, 1.0);

    let mut turns = Turns::default();
    assert!(worker.take_turn(&mut turns));
    assert!(
        !spool.job(1).join("answer.json").exists(),
        "answered only once its last part is in"
    );
    while worker.take_turn(&mut turns) {}

    assert_eq!(
        served(&log),
        ["job-1 0..1800", "job-2 0..11", "job-1 1800..2000"],
        "owner 8's page waits behind one turn of owner 7's long one, not all of it"
    );
    for job in [1, 2] {
        let answer = std::fs::read(spool.job(job).join("answer.json")).expect("answered");
        let answer: serde_json::Value = serde_json::from_slice(&answer).expect("JSON");
        assert_eq!(answer["failed"], json!([null]), "one page, one answer");
    }
    let _ = std::fs::remove_dir_all(&root);
}
