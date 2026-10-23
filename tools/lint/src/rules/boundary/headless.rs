//! The headless workspace never touches a display.
//!
//! `core` and `cli` never touch a display (their crate docs say so), and the desktop app is
//! its own workspace (`app/`) so a graphics stack never reaches the root one.
//! The root `Cargo.lock` is the whole of the root workspace's tree, direct and
//! transitive, so a GUI toolkit, a GPU surface, a window, a sound device or a
//! file dialog reaching it shows up there, whichever crate pulled it in.
//!
//! There is no inline exception: the lockfile is generated, so there is
//! nowhere to write one that would survive, and nothing in the root workspace
//! has a reason to want one.

use std::path::Path;

use super::super::Finding;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "headless";

/// The lockfile this rule reads, relative to the repository root.
pub const LOCKFILE: &str = "Cargo.lock";

/// Crates that only a program with a display has a reason to build: each is
/// matched by name, and by name followed by `-` or `_` (`egui_extras`).
pub const DISPLAY: [&str; 7] = ["egui", "eframe", "winit", "wgpu", "glow", "cpal", "rfd"];

/// The display crate `package` is, if any.
pub fn display_crate(package: &str) -> Option<&'static str> {
    DISPLAY.into_iter().find(|crate_| {
        package
            .strip_prefix(crate_)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['-', '_']))
    })
}

/// Every package in the lockfile `text` that is a display crate.
pub fn check(text: &str) -> Vec<Finding> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let package = line.strip_prefix("name = \"")?.strip_suffix('"')?;
            Some((index, package, display_crate(package)?))
        })
        .map(|(index, package, display)| Finding {
            path: Path::new(LOCKFILE).to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: format!(
                "`{package}` ({display}) is in the root workspace's tree, which never touches a \
                 display: a window belongs in `app/`, its own workspace"
            ),
        })
        .collect()
}
