//! Every command and every argument says what it is for.
//!
//! `--help` is the only documentation of the only interface scorsese has
//! today, and it is the one an agent reads at the moment it is deciding what
//! to run. A flag with no help is a capability that exists and cannot be
//! found: nothing fails, the agent simply never uses it.
//!
//! The tree is walked rather than listed, so a flag added tomorrow is covered
//! the day it is added — which is the whole point of checking this here
//! instead of remembering to write help text. The walk is shared with the
//! server's command line ([`walk`]).

#[path = "help/walk.rs"]
mod walk;

use clap::{Arg, CommandFactory};
use scorsese_cli::Cli;

#[test]
fn every_command_and_argument_has_help_text() {
    let found = walk::undocumented(Cli::command());
    assert!(
        found.is_empty(),
        "no help text on:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_walk_actually_reaches_the_tree() {
    let arguments = walk::arguments(Cli::command());
    assert!(
        arguments > 10,
        "walked only {arguments} arguments — the walk, not the CLI, is what \
         changed"
    );
}

#[test]
fn the_walk_finds_a_flag_and_a_command_with_nothing_to_say() {
    let bare = clap::Command::new("bare")
        .about("Documented.")
        .arg(Arg::new("quiet").long("quiet"))
        .subcommand(clap::Command::new("silent"));
    let found = walk::undocumented(bare);
    assert!(found.contains(&"bare: `quiet`".to_owned()), "{found:?}");
    assert!(
        found.contains(&"bare silent: the command itself".to_owned()),
        "{found:?}"
    );
}
