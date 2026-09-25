//! Which limit a given path falls under.
//!
//! The rules, stated once so they can be argued with rather than guessed at:
//!
//! - **Code is measured: `.rs`, and the web front-end's `.ts` and `.tsx`.**
//!   The cap exists because code is what gets read and edited to change
//!   behaviour; prose and data files are long for reasons that splitting them
//!   would not improve.
//! - **TypeScript takes the same two caps as Rust, and no exception.** `web/`
//!   is edited by the same agents, under the same constraint of holding one
//!   file in context at a time, and a React component past 300 lines of code is
//!   a component doing several things — the same smell a long Rust module is.
//!   A second pair of numbers would be a second rule to argue about with no
//!   reason behind the difference. shadcn/ui's components are included: shadcn
//!   copies them into `web/src/components/ui/` precisely so they are ours to
//!   edit, which makes them source like any other, and one that arrives over
//!   the cap is split or not vendored.
//! - **A file is a *test* file when a directory named `tests` is anywhere in
//!   its path.** That is the Cargo convention for integration-test targets
//!   (`crates/*/tests/**`), and it is the file's role in the build that decides
//!   the cap, not its subject matter. So `crates/golden/src/**` — test
//!   infrastructure, but compiled as a library others use — is *source*, and
//!   a helper module under `crates/render/tests/common/` is a *test* file.
//! - **A TypeScript file is also a test file when its name ends `.test.ts` or
//!   `.test.tsx`.** That is how `bun test` finds tests — beside the component
//!   they test, not in a `tests` directory — so it is the same rule: the file's
//!   role in the build decides the cap.
//! - **A `#[cfg(test)]` module inside a source file changes nothing.** The unit
//!   being capped is the file an agent has to hold in its head, and inline
//!   tests are part of that file. A source file with tests in it gets 300
//!   lines of code total, not 300 plus a test allowance.
//! - **`build.rs` is source.** It is compiled and run, and nothing about it
//!   makes 300 lines the wrong number.
//! - **Hidden directories and build output are not walked at all**: see
//!   [`is_skipped_dir`].
//!
//! There is no generated-code exception, because there is no generated Rust in
//! this repo. If some ever lands it belongs under a directory this module
//! skips, so the exception stays a location rather than a marker comment that
//! any file could grow.

use std::path::Path;

/// The cap on a source file, in lines of code.
pub const SOURCE_LIMIT: usize = 300;

/// The cap on a test file, in lines of code.
pub const TEST_LIMIT: usize = 150;

/// Which of the two caps a file is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Anything compiled into a library or binary, including inline
    /// `#[cfg(test)]` modules and `build.rs`.
    Source,
    /// An integration-test target or one of its helper modules — anything
    /// under a `tests` directory.
    Test,
}

impl Kind {
    /// The line limit for this kind.
    pub const fn limit(self) -> usize {
        match self {
            Self::Source => SOURCE_LIMIT,
            Self::Test => TEST_LIMIT,
        }
    }

    /// How the kind is named in a failure message.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Test => "test",
        }
    }
}

/// Directories the gate never descends into.
///
/// Hidden ones (`.git`, `.github`, and the `.claude/worktrees/` copies of the
/// repo that agents work in) hold history and tooling state rather than
/// project source — and counting a worktree's files would report every
/// violation several times over. The named ones are build output and
/// per-project scratch space: `target/`, and the `generated/`, `cache/`
/// directories of a `*.scor` project; and `node_modules/`, the web front-end's
/// installed packages, which ship `.ts` of their own. (`web/dist/` needs no
/// entry: a build writes `.js`, which is never measured.)
pub fn is_skipped_dir(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "target" | "generated" | "cache" | "node_modules")
}

/// Which cap `path` falls under, or `None` if the gate has no opinion about it
/// — not code, or inside a directory that is never walked.
///
/// `path` is interpreted relative to the scan root, so absolute paths from
/// outside it are answered on the same component rules and nothing else.
pub fn classify(path: &Path) -> Option<Kind> {
    let extension = path.extension()?.to_str()?;
    if !matches!(extension, "rs" | "ts" | "tsx") {
        return None;
    }

    let dirs = path.parent()?.components();
    let named_as_test = extension != "rs"
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with(".test"));
    let mut kind = if named_as_test {
        Kind::Test
    } else {
        Kind::Source
    };
    for dir in dirs.filter_map(|c| c.as_os_str().to_str()) {
        if is_skipped_dir(dir) {
            return None;
        }
        if dir == "tests" {
            kind = Kind::Test;
        }
    }

    Some(kind)
}
