//! A capture worker answering a test's spool, the way the capture container
//! answers the server's: the real binary's `capture-one`, its sandbox off as
//! `tests/suite/captures` has it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use scorsese_server::captures::Spool;
use scorsese_server::captures::worker::{Isolation, Worker};

/// A worker answering `spool` for the rest of the test, once it has said so.
pub(crate) fn answering(spool: &Spool) {
    scorsese_render::testing::require_chrome_env();
    let worker = Worker {
        spool: spool.clone(),
        isolation: Isolation::Process {
            program: PathBuf::from(env!("CARGO_BIN_EXE_scorsese-server")),
            sandbox: false,
            library: None,
        },
        deadline: scorsese_server::captures::deadline,
    };
    std::thread::spawn(move || worker.run());
    let alive = spool.root().join("worker.alive");
    let started = Instant::now();
    while !alive.is_file() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the worker beats"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
