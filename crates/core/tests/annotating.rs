//! The script and the notes: how they serialise, and in what order they read.
//!
//! Both fields are additive, so the thing most worth pinning is what a document
//! that says nothing looks like — a v9 project that gained neither field must
//! still round-trip to the same bytes it always did.

mod common;

use scorsese_core::{Annotated, Project, ProjectPath};

/// The fixture, with a script and a note on one of each annotatable thing.
fn annotated() -> Project {
    let mut project = common::project();
    project.script = Some(ProjectPath::new("script.md"));
    common::asset_mut(&mut project, "logo").note = Some("the client's own mark".to_owned());
    let track = project.tracks.first_mut().expect("a first track");
    track.note = Some("the plate everything else sits on".to_owned());
    track.clips.first_mut().expect("a first clip").note = Some("tried full-bleed first".to_owned());
    project
}

#[test]
fn a_script_and_notes_survive_a_round_trip() {
    let project = annotated();
    let reparsed = Project::from_json(&project.to_json().expect("serialise")).expect("reparse");
    assert_eq!(project, reparsed);
    assert_eq!(project.validate(), Ok(()));
}

/// The reason `skip_serializing_if` is on both: a project that explains nothing
/// about itself must not start carrying two empty fields that say it does.
#[test]
fn a_project_with_neither_writes_neither() {
    let json = common::project().to_json().expect("serialise");
    assert!(!json.contains("script"), "{json}");
    assert!(!json.contains("note"), "{json}");
}

#[test]
fn notes_read_in_document_order_assets_first() {
    let project = annotated();
    let found: Vec<(Annotated, &str)> = project.notes().map(|noted| (noted.on, noted.id)).collect();
    assert_eq!(
        found,
        [
            (Annotated::Asset, "logo"),
            (Annotated::Track, "v1"),
            (Annotated::Clip, "c-shot"),
        ],
        "assets, then each track followed by the clips on it"
    );
}

/// A note is a note about the element, not about the id — the two are carried
/// together because an id alone cannot say which table it came from.
#[test]
fn a_note_names_what_it_is_attached_to() {
    let project = annotated();
    let first = project.notes().next().expect("a first note");
    assert_eq!(
        first.to_string(),
        "asset `logo`: the client's own mark",
        "the element kind is part of the line, not left to the reader"
    );
}

#[test]
fn a_project_that_says_nothing_has_no_notes() {
    assert_eq!(common::project().notes().count(), 0);
}
