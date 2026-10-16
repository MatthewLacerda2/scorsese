//! The decided architecture's boundaries, held by a check instead of review
//! (#1224).
//!
//! `CLAUDE.md`'s crate map and each crate's `lib.rs` *Boundary* state them;
//! each is one `Cargo.toml` line or one import away from breaking silently,
//! so each is read here:
//!
//! - [`edges`] — which crate of ours may depend on which, and that only
//!   providers holds an HTTP client and only server talks to Postgres.
//! - [`headless`] — no display crate in the root workspace's lockfile.
//! - [`fences`] — words a crate's source may not write: zimmer, the
//!   compositor and core do no I/O they disclaim, render reads zimmer without
//!   synthesising, ffmpeg is spawned only by render's command builder, and one
//!   module of providers names an HTTP library.
//!
//! [`manifest`] reads the `Cargo.toml`s the first of them needs. Two
//! boundaries are held elsewhere, by the tool that sees them best: where an
//! environment variable may be read is clippy's `disallowed-methods`
//! (`clippy.toml`), and that no MCP tool names a client is a test over the
//! registry (`crates/mcp/tests/neutral.rs`).

pub mod edges;
pub mod fences;
pub mod headless;
pub mod manifest;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::Finding;

/// The directories whose children are crates with a manifest each.
const CRATE_PARENTS: [&str; 2] = ["crates", "tools"];

/// Crates that are a directory of their own rather than a child of one of
/// [`CRATE_PARENTS`].
const LONE_CRATES: [&str; 1] = ["app"];

/// Every crate manifest under `root`, relative to it, ordered by path. A
/// directory that is not there contributes nothing.
pub fn manifests(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found: Vec<PathBuf> = LONE_CRATES
        .iter()
        .map(|dir| Path::new(dir).join("Cargo.toml"))
        .collect();
    for parent in CRATE_PARENTS {
        let Ok(entries) = fs::read_dir(root.join(parent)) else {
            continue;
        };
        for entry in entries {
            found.push(
                Path::new(parent)
                    .join(entry?.file_name())
                    .join("Cargo.toml"),
            );
        }
    }
    found.retain(|path| root.join(path).is_file());
    found.sort();
    Ok(found)
}

/// Every finding of the rules that read manifests and the lockfile.
pub fn check_root(root: &Path) -> io::Result<Vec<Finding>> {
    let mut found = Vec::new();
    for path in manifests(root)? {
        let text = fs::read_to_string(root.join(&path))?;
        found.extend(edges::check(&path, &text, &manifest::parse(&text)));
    }
    if let Ok(lock) = fs::read_to_string(root.join(headless::LOCKFILE)) {
        found.extend(headless::check(&lock));
    }
    Ok(found)
}
