//! `Project::save`, which is where it matters.
//!
//! The failure guarded against here is not "the write went wrong" — it is a
//! reader arriving in the middle of a write that goes perfectly well.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use scorsese_core::{
    Asset, AssetId, Clip, ClipId, Fps, Frames, PROJECT_FILE_NAME, Project, Track, TrackId,
    TrackKind,
};

use crate::common;

/// A project big enough that writing it is not one atomic syscall by luck. A
/// hundred clips is an ordinary edit and several kilobytes of JSON.
fn sizeable(dir: &Path) -> Project {
    let mut project = Project::create(dir, Some("sizeable"), Fps::THIRTY).expect("create");
    project.assets.push(Asset::text(
        AssetId::new("title"),
        "a title long enough to take up room in the document",
    ));
    let clips = (0..100)
        .map(|n| {
            Clip::new(
                ClipId::new(format!("clip-{n}")),
                AssetId::new("title"),
                Frames(n * 10),
                Frames(10),
            )
        })
        .collect();
    project.tracks.push(Track {
        note: None,
        id: TrackId::new("v1"),
        kind: TrackKind::Video,
        name: None,
        clips,
    });
    project
}

#[test]
fn saving_over_a_project_replaces_it_whole() {
    let dir = common::temp_project_dir("save-replace");
    let mut project = sizeable(&dir);
    project.save(&dir).expect("first save");

    project.name = "renamed".to_owned();
    project.tracks.clear();
    project.save(&dir).expect("second save");

    let reloaded = Project::load(&dir).expect("the project still loads");
    assert_eq!(reloaded.name, "renamed");
    assert!(
        reloaded.tracks.is_empty(),
        "the new document replaced the old one rather than being written into it"
    );
}

/// The whole point. A reader arriving mid-write must never see a partial
/// document — and with `fs::write` it does, because that truncates its target
/// and then fills it.
///
/// This can only fail by observing a real partial read, never by chance, so a
/// pass is weak evidence and a failure is proof. Held against the old
/// `fs::write`, it fails almost immediately — the reader catches a zero-length
/// `project.json`, which is the truncate-then-fill gap itself. That is what
/// makes it worth keeping rather than a test that passes for no reason.
#[test]
fn a_reader_never_catches_a_project_half_written() {
    let dir = common::temp_project_dir("save-race");
    let project = sizeable(&dir);
    project
        .save(&dir)
        .expect("the first save, before anyone reads");

    let file = dir.join(PROJECT_FILE_NAME);
    let stop = Arc::new(AtomicBool::new(false));

    let reader = {
        let file = file.clone();
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut reads = 0_u32;
            while !stop.load(Ordering::Relaxed) {
                // A missing file would be its own failure: rename never leaves
                // the target absent, so `NotFound` here would mean the old
                // document was unlinked before the new one arrived.
                let json = std::fs::read_to_string(&file).expect("project.json is always there");
                Project::from_json(&json).expect("every read is a whole document");
                reads += 1;
            }
            reads
        })
    };

    for n in 0..300 {
        let mut edited = project.clone();
        edited.name = format!("save {n}");
        edited.save(&dir).expect("save");
    }
    stop.store(true, Ordering::Relaxed);

    let reads = reader.join().expect("the reader saw only whole documents");
    assert!(reads > 0, "the reader never got a look in");
}
