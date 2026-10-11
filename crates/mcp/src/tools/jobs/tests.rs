//! `jobs` with `wait` (#1056), over stand-in renders: it answers when the job
//! ends, at once when it already has, and a cancelled job ends the wait.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use scorsese_render::{Cancel, Progress};
use serde_json::{Value, json};

use super::Jobs;
use crate::renders::{Outcome, Renders, Work};
use crate::tools::{Context, Tool};

/// A render that ends with whatever is sent, or when cancelled.
fn held(renders: &Renders, out: &str) -> Sender<Outcome> {
    let (send, receive) = channel::<Outcome>();
    let work: Work = Box::new(move |_: Progress, cancel: Cancel| {
        loop {
            if cancel.is_cancelled() {
                return Err("cancelled after 3 of 9 frames".to_owned());
            }
            if let Ok(outcome) = receive.recv_timeout(Duration::from_millis(5)) {
                return outcome;
            }
        }
    });
    let project = Path::new("waits.scor");
    renders
        .start(project, (out, PathBuf::from(out)), Cancel::new(), work)
        .expect("starts");
    send
}

/// What `jobs` answers `arguments` with, or why it refused.
fn asked(renders: &Renders, cancel: &Cancel, arguments: Value) -> Result<String, String> {
    let mut context = Context::new(cancel, renders, None);
    let reply = Jobs.call_in(&arguments, &mut context)?;
    Ok(reply.parts[0].text.clone())
}

fn waiting(job: u64) -> Value {
    json!({ "project": "waits.scor", "job": job, "wait": true })
}

#[test]
fn a_running_job_is_answered_only_once_it_is_done() {
    let renders = Renders::default();
    let send = held(&renders, "cut.mp4");
    let finish = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        send.send(Ok("wrote cut.mp4 — 9 frames".to_owned()))
    });
    let said = asked(&renders, &Cancel::new(), waiting(1)).expect("answers");
    assert_eq!(said, "job 1 (render): done — wrote cut.mp4 — 9 frames");
    finish.join().expect("joined").expect("sent");
}

#[test]
fn a_finished_job_is_answered_at_once_as_without_wait() {
    let renders = Renders::default();
    let send = held(&renders, "cut.mp4");
    send.send(Err("ffmpeg refused".to_owned())).expect("sent");
    let job = renders.get(1).expect("started");
    job.ended_within(Duration::from_secs(10));
    let plain = asked(
        &renders,
        &Cancel::new(),
        json!({ "project": "waits.scor", "job": 1 }),
    );
    let waited = asked(&renders, &Cancel::new(), waiting(1));
    assert_eq!(waited, plain);
    assert_eq!(
        waited.as_deref(),
        Ok("job 1 (render): failed — ffmpeg refused")
    );
}

#[test]
fn a_job_cancelled_while_waited_on_ends_the_wait() {
    let renders = Renders::default();
    let _keep = held(&renders, "cut.mp4");
    let job = renders.get(1).expect("started");
    let stop = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        job.cancel();
    });
    let said = asked(&renders, &Cancel::new(), waiting(1)).expect("answers");
    assert_eq!(
        said,
        "job 1 (render): cancelled — cancelled after 3 of 9 frames"
    );
    stop.join().expect("joined");
}

#[test]
fn cancelling_the_wait_leaves_the_render_running() {
    let renders = Renders::default();
    let send = held(&renders, "cut.mp4");
    let cancel = Cancel::new();
    cancel.cancel();
    let said = asked(&renders, &cancel, waiting(1)).expect("answers");
    assert_eq!(said, "Stopped waiting for job 1; it renders on.");
    let job = renders.get(1).expect("started");
    assert!(
        job.ended_within(Duration::from_millis(50)).is_none(),
        "still rendering"
    );
    send.send(Ok("wrote".to_owned())).expect("sent");
}

#[test]
fn wait_without_a_job_is_refused_in_a_sentence() {
    let renders = Renders::default();
    let refused = asked(
        &renders,
        &Cancel::new(),
        json!({ "project": "a.scor", "wait": true }),
    );
    assert!(refused.is_err_and(|why| why.starts_with("wait waits for one job: give job")));
}
