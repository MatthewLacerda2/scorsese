//! The `.env` a checkout keeps, and how much of the convention it honours.

use std::fs;

use crate::common::scratch;
use scorsese_providers::credentials::Environment;

/// A directory with this `.env` written at the top of it, plus a child
/// directory to run "from" — because a command is as often run from a project
/// directory as from the checkout root.
fn checkout(contents: &str) -> std::path::PathBuf {
    let root = scratch("dotenv");
    let nested = root.join("teaser.scor");
    fs::create_dir_all(&nested).expect("create the directories");
    fs::write(root.join(".env"), contents).expect("write the .env");
    nested
}

#[test]
fn a_dot_env_above_the_working_directory_is_found() {
    let from = checkout("SCORSESE_FIXTURE_KEY=from-the-file\n");
    let environment = Environment::discover(&from);
    assert_eq!(
        environment.get("SCORSESE_FIXTURE_KEY"),
        Some("from-the-file")
    );
    assert!(environment.dotenv().is_some(), "and it says which file");
}

/// Everything such files conventionally allow, and nothing past it: no
/// interpolation, no escapes, no multi-line values. Past this point it is a
/// shell being reimplemented badly.
#[test]
fn it_honours_comments_export_and_quotes() {
    let from = checkout(
        "# a comment\n\n\
         export SCORSESE_FIXTURE_EXPORTED=one\n\
         SCORSESE_FIXTURE_QUOTED=\"two\"\n\
         SCORSESE_FIXTURE_SINGLE='three'\n\
         SCORSESE_FIXTURE_BLANK=\n",
    );
    let environment = Environment::discover(&from);
    assert_eq!(environment.get("SCORSESE_FIXTURE_EXPORTED"), Some("one"));
    assert_eq!(environment.get("SCORSESE_FIXTURE_QUOTED"), Some("two"));
    assert_eq!(environment.get("SCORSESE_FIXTURE_SINGLE"), Some("three"));
    assert_eq!(environment.get("SCORSESE_FIXTURE_BLANK"), None);
    assert_eq!(environment.get("# a comment"), None);
}

#[test]
fn no_dot_env_anywhere_is_not_a_failure() {
    let alone = scratch("no-dotenv");
    fs::create_dir_all(&alone).expect("create the directory");
    assert!(Environment::discover(&alone).dotenv().is_none());
}
