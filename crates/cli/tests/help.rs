//! Every command and every argument says what it is for.
//!
//! `--help` is the only documentation of the only interface scorsese has
//! today, and it is the one an agent reads at the moment it is deciding what
//! to run. A flag with no help is a capability that exists and cannot be
//! found: nothing fails, the agent simply never uses it.
//!
//! The tree is walked rather than listed, so a flag added tomorrow is covered
//! the day it is added — which is the whole point of checking this here
//! instead of remembering to write help text.

use clap::{Command, CommandFactory};
use scorsese_cli::cli::Cli;

/// One place with nothing to say for itself.
#[derive(Debug)]
struct Undocumented {
    /// The command it was found on, as a reader would type it.
    command: String,
    /// The argument, or the command itself.
    what: String,
}

/// Walks the command tree, collecting everything with no help text.
fn undocumented(command: &Command, path: &str, found: &mut Vec<Undocumented>) {
    let here = if path.is_empty() {
        command.get_name().to_owned()
    } else {
        format!("{path} {}", command.get_name())
    };

    if is_blank(command.get_about().map(ToString::to_string)) {
        found.push(Undocumented {
            command: here.clone(),
            what: "the command itself".to_owned(),
        });
    }

    for argument in command.get_arguments() {
        if is_blank(argument.get_help().map(ToString::to_string)) {
            found.push(Undocumented {
                command: here.clone(),
                what: format!("`{}`", argument.get_id()),
            });
        }
    }

    for sub in command.get_subcommands() {
        undocumented(sub, &here, found);
    }
}

/// Absent and present-but-empty are the same failure to a reader.
fn is_blank(help: Option<String>) -> bool {
    help.is_none_or(|help| help.trim().is_empty())
}

#[test]
fn every_command_and_argument_has_help_text() {
    // Built, not merely derived: `--help`, `--version` and the `help`
    // subcommand only exist after clap has assembled the tree, and they are
    // part of what a reader sees.
    let mut command = Cli::command();
    command.build();

    let mut found = Vec::new();
    undocumented(&command, "", &mut found);

    let listed: Vec<String> = found
        .iter()
        .map(|it| format!("{}: {}", it.command, it.what))
        .collect();
    assert!(
        found.is_empty(),
        "no help text on:\n  {}",
        listed.join("\n  ")
    );
}

#[test]
fn the_walk_actually_reaches_the_tree() {
    let mut command = Cli::command();
    command.build();

    let arguments: usize = count(&command);
    assert!(
        arguments > 10,
        "walked only {arguments} arguments — the walk, not the CLI, is what \
         changed"
    );
}

/// Every argument in the tree, however deep.
fn count(command: &Command) -> usize {
    command.get_arguments().count() + command.get_subcommands().map(count).sum::<usize>()
}
