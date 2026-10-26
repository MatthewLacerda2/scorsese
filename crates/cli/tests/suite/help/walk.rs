//! Walking a clap command tree for everything with no help text.
//!
//! Shared by every binary's help test — `scorsese`'s beside this file, and
//! `scorsese-server`'s, which includes it by path from
//! `crates/server/tests/suite/operator.rs` — so "every command and flag has help
//! text" means one walk for every command line the repo ships.
//! By path rather than through a crate, because a crate only tests share
//! would put a dependency edge between `cli` and `server` for one function.

use clap::Command;

/// Everything in `command`'s tree with nothing to say for itself, each as
/// `"<command path>: <what>"`. The tree is built first: `--help`,
/// `--version` and the `help` subcommand only exist after clap has assembled
/// it, and they are part of what a reader sees.
pub(crate) fn undocumented(mut command: Command) -> Vec<String> {
    command.build();
    let mut found = Vec::new();
    walk(&command, "", &mut found);
    found
}

/// Every argument in `command`'s built tree, however deep — so a test can
/// show the walk reached the tree, not an empty shell of it.
pub(crate) fn arguments(mut command: Command) -> usize {
    command.build();
    count(&command)
}

/// Walks the command tree, collecting everything with no help text.
fn walk(command: &Command, path: &str, found: &mut Vec<String>) {
    let here = if path.is_empty() {
        command.get_name().to_owned()
    } else {
        format!("{path} {}", command.get_name())
    };
    if is_blank(command.get_about().map(ToString::to_string)) {
        found.push(format!("{here}: the command itself"));
    }
    for argument in command.get_arguments() {
        if is_blank(argument.get_help().map(ToString::to_string)) {
            found.push(format!("{here}: `{}`", argument.get_id()));
        }
    }
    for sub in command.get_subcommands() {
        walk(sub, &here, found);
    }
}

/// Absent and present-but-empty are the same failure to a reader.
fn is_blank(help: Option<String>) -> bool {
    help.is_none_or(|help| help.trim().is_empty())
}

/// Every argument in the tree, however deep.
fn count(command: &Command) -> usize {
    command.get_arguments().count() + command.get_subcommands().map(count).sum::<usize>()
}
