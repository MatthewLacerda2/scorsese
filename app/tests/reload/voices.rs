//! What the window does when `scorsese voices` fills the cache beside it.
//!
//! The first run is the case: a project with no listing shows a picker with
//! nothing in it and a sentence naming the command that fixes that. Somebody
//! runs the command, in the terminal next to the window — and what the window
//! does next is the whole of this file.

use std::path::Path;

use egui_kittest::kittest::Queryable;

use crate::{SETTLE, fixture, window};

/// The sentence the picker shows when the project has no listing at all.
const EMPTY: &str = "nothing to pick from yet";

/// What the third column says once there is a list and no voice chosen yet.
const LIVE: &str = "required before this line can be spoken";

/// The whole point: a list fetched after the window opened reaches the picker,
/// with nobody pressing anything.
#[test]
fn a_listing_fetched_after_the_window_opened_reaches_the_picker() {
    let project = fixture::project("reload-voices");
    let mut harness = window(project.path());
    harness.state_mut().select("c-vo");
    harness.run();
    assert!(
        harness.query_by_label_contains(EMPTY).is_some(),
        "the picker has to start with nothing in it for this to mean anything"
    );

    fetch(project.path());
    std::thread::sleep(SETTLE);
    harness.run();

    assert!(
        harness.query_by_label_contains(EMPTY).is_none(),
        "the window went on saying there were no voices after one was fetched"
    );
    assert!(
        harness.query_by_label_contains(LIVE).is_some(),
        "the picker has a list now and has to read as one"
    );
}

/// `Look again` stays. It is rarely needed once the watch is doing the
/// noticing, and it is what there is to press when a look misses one.
#[test]
fn the_button_is_still_there_to_press() {
    let project = fixture::project("reload-voices-button");
    let mut harness = window(project.path());
    harness.state_mut().select("c-vo");
    harness.run();

    assert!(
        harness.query_by_label("Look again").is_some(),
        "the manual escape hatch went away"
    );
}

/// A listing landing in the project's cache, as `scorsese voices` writes one.
///
/// Written as text rather than through `scorsese-providers` for the reason the
/// document fixture is: what is on disk is the contract, and a test that writes
/// through our own writer cannot catch the two disagreeing.
fn fetch(project: &Path) {
    let dir = scorsese_providers::voices::cache_dir(project);
    std::fs::create_dir_all(&dir).expect("create cache/voices/");
    std::fs::write(
        dir.join("listing.json"),
        r#"{
  "listing": "library",
  "fetched_at": null,
  "fetched_unix": null,
  "voices": [ { "id": "v-ana", "name": "Ana", "traits": ["pt", "female"] } ]
}
"#,
    )
    .expect("write a listing");
}
