//! The picker's re-fit, on a project on disk: what it saves and what it says.

use scorsese_core::{Fps, Project, Ratio};

use super::{change, offered};
use crate::project::{Open, open};

/// A fresh 16:9 project on disk, opened as the window opens one.
fn opened(label: &str) -> Open {
    let root = crate::scratch::fresh(&format!("scorsese-app-framing-{label}"), ".scor");
    Project::create(&root, None, Fps::THIRTY, Ratio::WIDE).expect("create the project");
    let Ok(open) = open(&root) else {
        panic!("the project opens");
    };
    open
}

#[test]
fn a_new_shape_is_refitted_and_saved() {
    let mut open = opened("saved");
    let said = change(&mut open, Ratio::TALL);
    assert_eq!(open.project.aspect, Ratio::TALL);
    assert_eq!(said.headline, "Reframed from 16:9 to 9:16.");
    assert_eq!(said.unsettled, 0);
    let on_disk = Project::load(&open.root).expect("read back");
    assert_eq!(on_disk.aspect, Ratio::TALL, "the file is the model");
}

#[test]
fn the_shape_it_already_is_changes_nothing() {
    let mut open = opened("same");
    let said = change(&mut open, Ratio::WIDE);
    assert_eq!(
        said.headline,
        "The project is already 16:9; nothing changed."
    );
    assert_eq!(open.project.aspect, Ratio::WIDE);
}

#[test]
fn the_four_shapes_are_offered_and_the_projects_own_too() {
    let named =
        |now: Ratio| -> Vec<String> { offered(now).iter().map(ToString::to_string).collect() };
    assert_eq!(named(Ratio::TALL), ["16:9", "9:16", "1:1", "4:5"]);
    let scope: Ratio = "21:9".parse().expect("a shape");
    assert_eq!(named(scope), ["16:9", "9:16", "1:1", "4:5", "7:3"]);
}
