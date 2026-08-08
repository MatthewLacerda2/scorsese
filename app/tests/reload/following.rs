//! What the window does when the document changes underneath it.

use scorsese_core::{ClipId, Frames};

use crate::{SETTLE, fixture, has_clip, rewrite, window};

/// The whole point: an edit made by something else reaches an open window,
/// without anyone relaunching it.
#[test]
fn an_edit_made_outside_the_window_arrives_in_it() {
    let project = fixture::project("reload-outside");
    let mut harness = window(project.path());
    harness.run();
    assert!(!has_clip(&harness, "c-extra"), "it is not there yet");

    rewrite(project.path(), |document| {
        document.replace(
            r#"{ "id": "c-title", "asset": "title", "start": 300, "duration": 120 } ] },"#,
            r#"{ "id": "c-title", "asset": "title", "start": 300, "duration": 120 },
      { "id": "c-extra", "asset": "title", "start": 460, "duration": 60 } ] },"#,
        )
    });
    std::thread::sleep(SETTLE);
    harness.run();

    assert!(
        has_clip(&harness, "c-extra"),
        "the outside edit never arrived"
    );
    assert_eq!(
        harness.state().disk_note().as_deref(),
        Some("reloaded — changed on disk"),
        "and the window has to say why what it shows changed"
    );
}

/// A reload that jumped to frame zero would be barely better than relaunching
/// the window, which is the thing this exists to stop anyone having to do.
#[test]
fn a_reload_leaves_the_viewer_where_it_was_standing() {
    let project = fixture::project("reload-viewer");
    let mut harness = window(project.path());
    harness.state_mut().scrub(Frames(137));
    harness.state_mut().select("c-title");
    harness.state_mut().also_select("c-shot");
    harness.run();

    rewrite(project.path(), |document| {
        document.replace(r#""name": "Narrated teaser""#, r#""name": "Renamed""#)
    });
    std::thread::sleep(SETTLE);
    harness.run();

    assert_eq!(
        harness.state().showing().expect("open").name,
        "Renamed",
        "the reload has to have happened for this test to mean anything"
    );
    assert_eq!(
        harness.state().playhead(),
        Frames(137),
        "the playhead moved"
    );
    assert_eq!(
        harness.state().selected(),
        [&ClipId::new("c-shot"), &ClipId::new("c-title")],
        "the selection was lost — all of it has to survive, not the last click"
    );
}

/// A document caught in a bad state is ordinary when something else is
/// writing. It is not a crash and not an empty window.
#[test]
fn a_document_that_will_not_load_leaves_the_last_good_one_on_screen() {
    let project = fixture::project("reload-broken");
    let mut harness = window(project.path());
    harness.run();
    let good = harness.state().showing().expect("open").clone();

    rewrite(project.path(), |_| "{ not a document".to_owned());
    std::thread::sleep(SETTLE);
    harness.run();

    assert_eq!(
        harness.state().showing(),
        Some(&good),
        "the last good document must stay on screen"
    );
    let note = harness
        .state()
        .disk_note()
        .expect("the window has to say so");
    assert!(note.contains("last good version"), "got {note}");
}

/// A document that **parses** and does not validate gets the same treatment on
/// a re-read, and that is the line rather than an oversight.
///
/// Opening one of these shows it read-only, because an empty window is the only
/// alternative. Re-reading into one has a better alternative right there on
/// screen — the version that worked — and the writer that produced this state
/// is very likely still mid-edit.
#[test]
fn a_document_that_stops_validating_leaves_the_last_good_one_on_screen() {
    let project = fixture::project("reload-invalid");
    let mut harness = window(project.path());
    harness.run();
    let good = harness.state().showing().expect("open").clone();

    rewrite(project.path(), |document| {
        document.replace(r#""asset": "bed""#, r#""asset": "ghost""#)
    });
    std::thread::sleep(SETTLE);
    harness.run();

    assert_eq!(
        harness.state().showing(),
        Some(&good),
        "the last good document must stay on screen"
    );
    let note = harness
        .state()
        .disk_note()
        .expect("the window has to say so");
    assert!(note.contains("1 problem"), "got {note}");
    assert!(note.contains("last good version"), "got {note}");
}

/// The window's own save looks like an outside edit from the filesystem's
/// point of view: the file is replaced, so its size and modification time both
/// move. Announcing that would make every drag look like somebody else's work.
///
/// The rewrite here changes the **bytes** without changing the **document** —
/// same JSON, differently laid out — so the poll genuinely fires and the
/// window genuinely re-reads. What it must not do is claim anything changed.
/// A version of this that wrote the file back byte-for-byte would pass without
/// the poll ever firing, and prove nothing.
#[test]
fn rewriting_the_same_document_is_not_somebody_elses_edit() {
    let project = fixture::project("reload-same");
    let mut harness = window(project.path());
    harness.run();

    let file = project.path().join("project.json");
    let before = std::fs::metadata(&file).expect("stat").len();
    // Newlines are structural in JSON — none of them is inside a string — so
    // indenting every line again is a different file and the same document.
    rewrite(project.path(), |document| document.replace('\n', "\n  "));
    let after = std::fs::metadata(&file).expect("stat").len();
    assert_ne!(before, after, "the file has to actually differ on disk");

    std::thread::sleep(SETTLE);
    harness.run();

    assert_eq!(
        harness.state().disk_note(),
        None,
        "the same document, however it is spelled, is not news"
    );
}
