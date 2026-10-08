//! A page loading its own project's media (#857). The server lays each media
//! file out as a link into its owner's library, so the capture must follow
//! those links there, and only there.

use std::os::unix::fs::symlink;
use std::time::Duration;

use scorsese_render::Cancel;
use scorsese_server::captures::dispatch::Pages;
use scorsese_server::captures::worker::Isolation;

use super::{spool, whole, worker};

/// Every `capture.json` under `dir`, read as text.
fn records_in(dir: &std::path::Path) -> String {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return String::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                records_in(&path)
            } else if path.file_name().is_some_and(|name| name == "capture.json") {
                std::fs::read_to_string(path).expect("a record")
            } else {
                String::new()
            }
        })
        .collect()
}

#[test]
fn a_page_loads_its_owners_media_through_the_library_and_nobody_elses() {
    let page = "<img src='../assets/mine.png'><img src='../assets/theirs.png'>";
    let (spool, root) = spool("media", page);
    let library = root.join("library");
    for user in ["7", "8"] {
        let theirs = library.join("users").join(user).join("library");
        std::fs::create_dir_all(&theirs).expect("a library");
        std::fs::write(theirs.join("abc.png"), user).expect("a file");
    }
    let pages = Pages {
        spool: spool.clone(),
        job: 1,
        cache: spool.root().join("pages/7/9"),
    };
    let assets = pages.folder().join("assets");
    std::fs::create_dir_all(&assets).expect("assets/");
    let file = |user: &str| library.join("users").join(user).join("library/abc.png");
    symlink(file("7"), assets.join("mine.png")).expect("a link");
    symlink(file("8"), assets.join("theirs.png")).expect("a link");
    let mut worker = worker(&spool, scorsese_server::captures::deadline);
    if let Isolation::Process { library: named, .. } = &mut worker.isolation {
        *named = Some(library.clone());
    }
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
        .capture(&[whole(0.5)], &Cancel::new())
        .expect("not cancelled");
    answering.join().expect("the worker answered");

    assert!(captured.failed.is_empty(), "{:?}", captured.failed);
    let record = records_in(&pages.cache);
    assert!(record.contains("\"assets/mine.png\""), "{record}");
    assert!(!record.contains("mine.png from outside"), "{record}");
    assert!(record.contains("theirs.png from outside"), "{record}");
    let _ = std::fs::remove_dir_all(root);
}
