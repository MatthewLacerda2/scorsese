//! `scorsese-lint [ROOT]` — fail if any code file is over its code-line limit,
//! or breaks one of the [`scorsese_lint::rules`].
//!
//! Defaults to the current directory, which is the repo root when CI runs it
//! as `cargo run --manifest-path tools/lint/Cargo.toml`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use scorsese_lint::{SOURCE_LIMIT, TEST_LIMIT, check_dir, check_tree};

fn main() -> ExitCode {
    let root = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let sized = size(&root);
    let ruled = rules(&root);
    if sized && ruled {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The size gate, reported; whether it passed.
fn size(root: &Path) -> bool {
    let report = match check_dir(root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("size gate: cannot read {}: {error}", root.display());
            return false;
        }
    };

    if report.is_clean() {
        println!(
            "size gate: {} files, all within {SOURCE_LIMIT} lines of code (source) / {TEST_LIMIT} (test)",
            report.checked
        );
        return true;
    }

    eprintln!(
        "size gate: {} of {} files are over the limit.",
        report.violations.len(),
        report.checked
    );
    for violation in &report.violations {
        eprintln!("  {violation}");
    }
    eprintln!("Split them by concern — nothing is grandfathered.");
    false
}

/// The conventions, reported; whether they all held.
fn rules(root: &Path) -> bool {
    let findings = match check_tree(root) {
        Ok(findings) => findings,
        Err(error) => {
            eprintln!("conventions: cannot read {}: {error}", root.display());
            return false;
        }
    };
    if findings.is_empty() {
        println!(
            "conventions: prefix groups, markers, skipped tests, crate docs, ignored code, schema literals -- all held"
        );
        return true;
    }
    eprintln!("conventions: {} findings.", findings.len());
    for finding in &findings {
        eprintln!("  {finding}");
    }
    eprintln!(
        "Fix each, or excuse it inline: `lint: allow(<rule>) <why>` on or just above the line."
    );
    false
}
