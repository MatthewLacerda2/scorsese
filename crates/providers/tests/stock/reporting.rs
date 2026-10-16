//! What an import says (#1289): the same lines on every surface, the
//! attribution and the licence among them, and whether to save.

use scorsese_providers::stock::{Choice, Imports, LICENCE, Medium, cache_dir, import};

use crate::common::project;
use crate::fake::{Fake, probe};

fn videos(ids: &[u64]) -> Vec<Choice> {
    let video = |&id| Choice {
        medium: Medium::Video,
        id,
    };
    ids.iter().map(video).collect()
}

#[test]
fn an_import_names_its_source_its_author_its_failures_and_the_licence() {
    let (root, mut project) = project("stock-report");
    let (cache, library) = (cache_dir(&root), Fake::with(10));
    let choices = videos(&[4, 99]);
    let frame = (1920, 1080);
    let answers = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choices,
        frame,
        &probe(),
    );
    let imports = Imports::new(&choices, answers, frame);

    assert!(imports.needs_save());
    let lines = imports.lines().expect("one came in");
    assert!(
        lines[0].starts_with("pixabay-4 — video 4 (medium, "),
        "{lines:?}"
    );
    assert!(
        lines[0].ends_with("by someone — https://pixabay.com/videos/id-4/"),
        "{lines:?}"
    );
    assert!(lines[1].starts_with("video 99 — failed: "), "{lines:?}");
    assert_eq!(lines[2], format!("From Pixabay. {LICENCE}"));

    let again = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choices[..1],
        frame,
        &probe(),
    );
    let again = Imports::new(&choices[..1], again, frame);
    assert!(!again.needs_save(), "nothing new came in");
    let lines = again.lines().expect("it is there");
    assert!(lines[0].contains("already in the project"), "{lines:?}");
}

#[test]
fn nothing_imported_is_a_refusal_naming_each_failure() {
    let (root, mut project) = project("stock-report-none");
    let choices = videos(&[98, 99]);
    let frame = (1920, 1080);
    let answers = import(
        &mut project,
        &root,
        &cache_dir(&root),
        &Fake::with(3),
        &choices,
        frame,
        &probe(),
    );
    let imports = Imports::new(&choices, answers, frame);
    assert!(!imports.needs_save());
    let refused = imports.lines().expect_err("nothing came in");
    assert!(refused.starts_with("video 98 — failed: "), "{refused}");
    assert!(refused.contains("; video 99 — failed: "), "{refused}");
    assert!(refused.ends_with(" — nothing was imported"), "{refused}");
}
