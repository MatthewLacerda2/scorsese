//! Page captures through the spool (#778): the server's side asking, the
//! worker answering, and the deadline the worker holds a page to.
//!
//! These run the real browser (`SCORSESE_CHROME`) through the real binary's
//! `capture-one`, and fail without it, as the render crate's page tests do. The
//! sandbox is off: tests run as root, where Chromium refuses one — the capture
//! container is where it is on, and #778's PR has that run.

use std::path::{Path, PathBuf};
use std::time::Duration;

use scorsese_core::Fps;
use scorsese_render::Cancel;
use scorsese_render::Resolution;
use scorsese_render::page::Request;
use scorsese_server::captures::dispatch::Pages;
use scorsese_server::captures::worker::{Isolation, Worker};
use scorsese_server::captures::{PROJECT, Spool};
use scorsese_server::renders::evict::pages::touch;

/// A spool of its own, with job 1's project holding `pages/page.html`.
fn spool(name: &str, html: &str) -> (Spool, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("scorsese-captures-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let spool = Spool::new(&root);
    let project = spool.job(1).join(PROJECT);
    std::fs::create_dir_all(project.join("pages")).expect("pages/");
    std::fs::create_dir_all(project.join("cache")).expect("cache/");
    std::fs::write(project.join("pages/page.html"), html).expect("a page");
    (spool, root)
}

fn worker(spool: &Spool, deadline: fn(u64) -> Duration) -> Worker {
    Worker {
        spool: spool.clone(),
        isolation: Isolation::Process {
            program: PathBuf::from(env!("CARGO_BIN_EXE_scorsese-server")),
            sandbox: false,
        },
        deadline,
    }
}

fn request(duration: f64) -> Request {
    Request {
        page: "pages/page.html".into(),
        resolution: Resolution::new(64, 36).expect("a raster"),
        fps: Fps::new(10, 1).expect("a rate"),
        duration,
    }
}

fn frames_in(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                frames_in(&path)
            } else {
                usize::from(path.file_name().is_some_and(|name| name == "frames.mkv"))
            }
        })
        .sum()
}

#[test]
fn a_page_asked_for_by_the_server_is_captured_by_the_worker_into_the_projects_cache() {
    let (spool, root) = spool("asked", "<body style='background:#0f0'></body>");
    let pages = Pages {
        spool: spool.clone(),
        job: 1,
        cache: root.join("pages/7/9"),
    };
    let worker = worker(&spool, scorsese_server::captures::deadline);
    let answering = std::thread::spawn(move || {
        let job = loop {
            match worker.next() {
                Some(job) => break job,
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        };
        worker.serve(&job);
    });

    let captured = pages
        .capture(&[request(0.5)], &Cancel::new())
        .expect("not cancelled");
    answering.join().expect("the worker answered");

    assert!(captured.chrome.is_some(), "the server knows the build");
    assert!(captured.failed.is_empty(), "{:?}", captured.failed);
    assert_eq!(frames_in(&pages.cache), 1, "kept in the project's cache");
    assert!(
        pages.folder().join("cache").is_symlink(),
        "the job's cache/ is the project's"
    );

    // A render reusing the capture marks it used, so eviction counts its age
    // from then (#849).
    let chrome = captured.chrome.expect("checked above");
    let frames = std::fs::read_dir(pages.cache.join("pages"))
        .expect("slots")
        .flatten()
        .map(|slot| slot.path().join("frames.mkv"))
        .find(|frames| frames.is_file())
        .expect("the capture's frames");
    let long_ago = std::time::SystemTime::now() - Duration::from_secs(49 * 3600);
    let file = std::fs::File::options().append(true).open(&frames);
    file.expect("frames").set_modified(long_ago).expect("aged");
    touch(&pages.folder(), &[request(0.5)], chrome.version());
    let age = std::fs::metadata(&frames).and_then(|meta| meta.modified());
    assert!(age.expect("a time") > long_ago + Duration::from_secs(3600));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_page_that_runs_past_its_deadline_is_stopped_and_said_to_be() {
    // Each frame takes a second of the page's own work, so it answers every
    // time and the capture's patience never fires: only the deadline can.
    let busy = "<script>requestAnimationFrame(function f() { \
                const s = performance.now(); while (performance.now() - s < 1000) {} \
                requestAnimationFrame(f); });</script>";
    let (spool, root) = spool("deadline", busy);
    let asked = serde_json::json!({ "requests": [
        { "page": "pages/page.html", "width": 64, "height": 36,
          "fps": { "num": 10, "den": 1 }, "duration": 60.0 }
    ] });
    let job = spool.job(1);
    std::fs::write(job.join("ask.json"), asked.to_string()).expect("ask");
    let worker = worker(&spool, |_| Duration::from_secs(3));

    let started = std::time::Instant::now();
    worker.serve(&job);

    let answer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(job.join("answer.json")).expect("an answer"))
            .expect("JSON");
    let why = answer["failed"][0].as_str().expect("it failed");
    assert!(why.contains("stopped after 3 seconds"), "{why}");
    assert!(started.elapsed() < Duration::from_secs(20), "and promptly");
    let _ = std::fs::remove_dir_all(root);
}
