use std::time::Instant;

use scorsese_core::Fps;
use scorsese_render::Resolution;
use scorsese_render::page::Request;

use super::*;
use crate::renders::{Quota, RenderCache};

/// A still's pages over a spool whose worker said it was there a moment ago,
/// and a project laid out with one page; the folder the project is laid in.
fn asking(label: &str) -> (StillPages, PathBuf) {
    // Dispatch reads only SCORSESE_CHROME: without it nothing reaches a job.
    scorsese_render::testing::require_chrome_env();
    let root = crate::scratch::dir(label);
    let spool = Spool::new(root.join("spool"));
    std::fs::create_dir_all(spool.root()).unwrap();
    std::fs::write(
        spool.alive(),
        scorsese_core::Timestamp::unix_now_or_epoch().to_string(),
    )
    .unwrap();
    let renders = RenderCache::new(root.join("renders"), Quota::bytes(u64::MAX));
    let pages = StillPages {
        cache: root.join("renders/pages"),
        spool,
        call: 1,
        pin: Arc::new(renders.pin(Path::new("pages"))),
    };
    let project = root.join("project");
    std::fs::create_dir_all(project.join("pages")).unwrap();
    std::fs::write(project.join("pages/p.html"), "<body></body>").unwrap();
    (pages, project)
}

fn of(frames: std::ops::Range<u64>) -> Wanted {
    let request = Request {
        page: "pages/p.html".into(),
        resolution: Resolution::new(64, 36).unwrap(),
        fps: Fps::THIRTY,
        duration: 1000.0,
        clips: Default::default(),
        words: Default::default(),
    };
    Wanted::new(request, frames)
}

#[test]
fn a_still_stopped_while_it_waits_leaves_its_job_to_finish_with_the_cache_held() {
    let (pages, project) = asking("still-stopped-waiting");
    let job = pages.spool.still(pages.call);
    let cancel = Cancel::new();
    let stopping = {
        let (cancel, asked) = (cancel.clone(), job.join(ASK));
        std::thread::spawn(move || {
            while !asked.exists() {
                std::thread::sleep(Duration::from_millis(20));
            }
            cancel.cancel();
        })
    };

    let ahead = pages.capture(&project, &[of(13..18)], &cancel);
    stopping.join().unwrap();

    assert!(job.join(ASK).is_file(), "left to the launcher");
    assert_eq!(Arc::strong_count(&pages.pin), 2, "the cache stays held");
    assert!(ahead.failed.contains_key("pages/p.html"), "{ahead:?}");
    // Once the job goes, nothing holds the cache any more (#1469).
    std::fs::remove_dir_all(&job).unwrap();
    let since = Instant::now();
    while Arc::strong_count(&pages.pin) > 1 && since.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(Arc::strong_count(&pages.pin), 1, "let go once the job went");
}

#[test]
fn a_still_stopped_before_it_asks_copies_and_launches_nothing() {
    let (pages, project) = asking("still-stopped-before");
    let cancel = Cancel::new();
    cancel.cancel();

    let ahead = pages.capture(&project, &[of(13..18)], &cancel);

    assert!(!pages.spool.still(pages.call).exists(), "no job asked");
    assert_eq!(Arc::strong_count(&pages.pin), 1, "nothing left waiting");
    assert_eq!(ahead.failed["pages/p.html"], STOPPED);
}

#[test]
fn a_still_waits_no_longer_than_its_patience() {
    assert_eq!(patience(&[of(13..18)]), deadline(18));
    // An instant late in a long page would give the launcher well past
    // nginx's five minutes.
    assert!(deadline(29_000) > PATIENCE);
    assert_eq!(patience(&[of(28_990..29_000)]), PATIENCE);
}

#[test]
fn a_link_a_capture_left_for_the_kept_pages_folder_is_removed_not_read() {
    let root = crate::scratch::dir("still-planted-pages");
    let (kept, project, outside) = (
        root.join("kept"),
        root.join("project"),
        root.join("outside"),
    );
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::create_dir_all(outside.join("slot")).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::os::unix::fs::symlink(&outside, kept.join(page::CACHE_PAGES)).unwrap();

    read_cache(&project, &kept).unwrap();

    let pages = kept.join(page::CACHE_PAGES);
    assert!(pages.is_dir() && !pages.is_symlink(), "a folder of its own");
    assert!(outside.join("slot").is_dir(), "what it led to is untouched");
    let read = project.join(CACHE_DIR).join(page::CACHE_PAGES);
    assert_eq!(
        std::fs::read_link(&read).unwrap(),
        std::path::absolute(&pages).unwrap()
    );
    assert_eq!(std::fs::read_dir(&read).unwrap().count(), 0);
}
