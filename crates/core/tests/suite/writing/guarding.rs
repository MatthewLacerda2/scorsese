//! The compare-and-swap on a save: two writers of one document, and only the
//! one that read what is actually there gets to publish.
//!
//! The loss this guards against is not a half-written file — `atomically`
//! closes that one. It is a whole, valid document landing on another whole,
//! valid document and taking its contents with it, with neither writer told.

use scorsese_core::{Baseline, Fps, PROJECT_FILE_NAME, Project, SaveError, fingerprint_of};

use crate::common;

/// The case from the issue, with the minutes taken out: two callers load the
/// same project, both change it, and the second one saved is refused rather
/// than dropping the first one's work.
#[test]
fn a_save_built_on_a_read_something_else_overtook_is_refused() {
    let (dir, _) = common::new_project("guard-stale");
    let mut first = Project::load(&dir).expect("the first reader");
    let mut second = Project::load(&dir).expect("the second reader");

    first.name = "the first writer's edit".to_owned();
    first.save(&dir).expect("the first save publishes");

    second.name = "the second writer's edit".to_owned();
    let refused = second.save(&dir).expect_err("the second save is refused");

    assert!(
        matches!(refused, SaveError::Stale { .. }),
        "got {refused:?}"
    );
    assert_eq!(
        Project::load(&dir).expect("the project still loads").name,
        "the first writer's edit",
        "the refused save must have written nothing at all"
    );
}

/// An agent that cannot recover from the message alone has been told nothing
/// useful, so the refusal has to name the project and say what to do next.
#[test]
fn the_refusal_names_the_project_and_says_to_read_it_again() {
    let (dir, _) = common::new_project("guard-message");
    let stale = Project::load(&dir).expect("read");
    let mut other = Project::load(&dir).expect("read");
    other.name = "somebody else got there first".to_owned();
    other.save(&dir).expect("somebody else publishes");

    let said = stale.save(&dir).expect_err("refused").to_string();

    assert!(said.contains(PROJECT_FILE_NAME), "got {said}");
    assert!(
        said.to_lowercase().contains("read the project again"),
        "got {said}"
    );
}

/// A save that is not preceded by anyone else's is the ordinary case, and the
/// guard must not so much as slow it down.
#[test]
fn reading_and_saving_in_turn_is_never_refused() {
    let (dir, _) = common::new_project("guard-sequential");
    for round in 0..3 {
        let mut project = Project::load(&dir).expect("read");
        project.name = format!("round {round}");
        project.save(&dir).expect("save");
    }
    assert_eq!(
        Project::load(&dir).expect("load").name,
        "round 2",
        "every save in the sequence landed"
    );
}

/// `project_new`, and the first save of a document nobody read from anywhere:
/// there is nothing on disk to lose, so there is nothing to refuse.
#[test]
fn a_first_save_into_a_directory_with_no_document_is_allowed() {
    let dir = common::temp_project_dir("guard-first");
    let project = common::project();

    project
        .save(&dir)
        .expect("nothing is there to be overwritten");

    assert!(dir.join(PROJECT_FILE_NAME).is_file());
    project.save(&dir).expect("and the same writer saves again");
}

/// The `project_write` path with no fingerprint: a document built from a
/// string is not a change to anything, so saving it over a project that exists
/// would be overwriting blind.
#[test]
fn a_document_that_was_never_read_from_the_file_is_refused() {
    let (dir, _) = common::new_project("guard-unread");
    let unread = Project::new("handed over", Fps::THIRTY);

    let refused = unread.save(&dir).expect_err("refused");

    assert!(
        matches!(refused, SaveError::Unread { .. }),
        "got {refused:?}"
    );
    assert!(
        Project::load(&dir).expect("load").name != "handed over",
        "the refused save must have written nothing"
    );
}

/// The other half of that path: a caller that read the document elsewhere says
/// which bytes it read, and is taken at its word only while they are still the
/// bytes on disk.
#[test]
fn a_document_carrying_the_fingerprint_of_what_is_there_is_allowed() {
    let (dir, _) = common::new_project("guard-claimed");
    let file = dir.join(PROJECT_FILE_NAME);
    let read = std::fs::read(&file).expect("read the document");

    let mut project = Project::from_json(&String::from_utf8(read.clone()).expect("utf-8"))
        .expect("the document parses");
    project.baseline = Baseline::claimed(fingerprint_of(&read));
    project.name = "written back".to_owned();
    project
        .save(&dir)
        .expect("the fingerprint is the one on disk");

    assert_eq!(Project::load(&dir).expect("load").name, "written back");

    let mut behind = Project::from_json(&String::from_utf8(read).expect("utf-8")).expect("parses");
    behind.baseline = Baseline::claimed(fingerprint_of(b"some other document"));
    let refused = behind
        .save(&dir)
        .expect_err("a stale fingerprint is refused");
    assert!(
        matches!(refused, SaveError::Stale { .. }),
        "got {refused:?}"
    );
}
