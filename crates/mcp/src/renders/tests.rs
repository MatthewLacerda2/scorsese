//! The job table with stand-in work, so every state is reached without ffmpeg.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use scorsese_render::{Cancel, Progress};

use super::{Job, Outcome, Renders, State, Work, line};

const LONG: Duration = Duration::from_secs(10);

/// Work that ends with whatever `outcome` is sent, or when cancelled.
fn held() -> (Work, std::sync::mpsc::Sender<Outcome>) {
    let (send, receive): (_, Receiver<Outcome>) = channel();
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
    (work, send)
}

fn start(
    renders: &Renders,
    project: &Path,
    out: &str,
    work: Work,
) -> Result<std::sync::Arc<Job>, String> {
    renders.start(project, (out, PathBuf::from(out)), Cancel::new(), work)
}

#[test]
fn a_job_runs_then_says_what_it_wrote() {
    let renders = Renders::default();
    let (work, send) = held();
    let job = start(&renders, Path::new("a.scor"), "cut.mp4", work).expect("starts");
    assert_eq!(job.id, 1);
    assert!(matches!(job.state(), State::Running(_)));
    assert!(
        line(&job).starts_with("job 1 (render): running, 0% — "),
        "{}",
        line(&job)
    );
    assert!(line(&job).ends_with("writing cut.mp4"), "{}", line(&job));
    send.send(Ok("wrote cut.mp4 — 9 frames".to_owned()))
        .expect("sent");
    assert_eq!(
        job.ended_within(LONG),
        Some(Ok("wrote cut.mp4 — 9 frames".to_owned()))
    );
    assert_eq!(
        line(&job),
        "job 1 (render): done — wrote cut.mp4 — 9 frames"
    );
}

#[test]
fn a_failure_and_a_cancel_read_differently() {
    let renders = Renders::default();
    let (work, send) = held();
    let failed = start(&renders, Path::new("a.scor"), "one.mp4", work).expect("starts");
    send.send(Err("ffmpeg refused".to_owned())).expect("sent");
    failed.ended_within(LONG);
    assert_eq!(line(&failed), "job 1 (render): failed — ffmpeg refused");

    let (work, _keep) = held();
    let stopped = start(&renders, Path::new("a.scor"), "two.mp4", work).expect("starts");
    stopped.cancel();
    stopped.ended_within(LONG);
    assert_eq!(
        line(&stopped),
        "job 2 (render): cancelled — cancelled after 3 of 9 frames"
    );
}

#[test]
fn one_file_is_written_by_one_render_at_a_time() {
    let renders = Renders::default();
    let (work, send) = held();
    let first = start(&renders, Path::new("a.scor"), "cut.mp4", work).expect("starts");
    let (again, _keep) = held();
    let refused = start(&renders, Path::new("a.scor"), "cut.mp4", again).err();
    assert!(refused.is_some_and(|why| why.starts_with("job 1 is still writing cut.mp4")));
    send.send(Ok("wrote".to_owned())).expect("sent");
    first.ended_within(LONG);
    let (after, _keep) = held();
    assert!(start(&renders, Path::new("a.scor"), "cut.mp4", after).is_ok());
}

#[test]
fn jobs_are_found_by_id_and_listed_newest_first_for_their_project() {
    let renders = Renders::default();
    let mut keep = Vec::new();
    for (project, out) in [
        ("a.scor", "1.mp4"),
        ("b.scor", "2.mp4"),
        ("a.scor", "3.mp4"),
    ] {
        let (work, send) = held();
        keep.push(send);
        start(&renders, Path::new(project), out, work).expect("starts");
    }
    let ids: Vec<u64> = renders
        .of(Path::new("a.scor"))
        .iter()
        .map(|j| j.id)
        .collect();
    assert_eq!(ids, vec![3, 1]);
    assert_eq!(
        renders.get(2).map(|job| job.out.clone()),
        Some("2.mp4".to_owned())
    );
    assert!(renders.get(4).is_none());
}

#[test]
fn ending_the_session_stops_every_render_and_waits_for_it() {
    let renders = Renders::default();
    let (work, _keep) = held();
    let job = start(&renders, Path::new("a.scor"), "cut.mp4", work).expect("starts");
    drop(renders);
    assert!(
        matches!(job.state(), State::Cancelled(_)),
        "stopped, and over"
    );
}
