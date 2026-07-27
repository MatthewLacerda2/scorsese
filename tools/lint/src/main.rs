//! `size-gate [ROOT]` — fail if any Rust file is over its line limit.
//!
//! Defaults to the current directory, which is the repo root when CI runs it
//! as `cargo run --manifest-path tools/lint/Cargo.toml`.

use std::path::PathBuf;
use std::process::ExitCode;

use scorsese_lint::{SOURCE_LIMIT, TEST_LIMIT, check_dir};

fn main() -> ExitCode {
    let root = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);

    let report = match check_dir(&root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("size gate: cannot read {}: {error}", root.display());
            return ExitCode::FAILURE;
        }
    };

    if report.is_clean() {
        println!(
            "size gate: {} files, all within {SOURCE_LIMIT} lines (source) / {TEST_LIMIT} lines (test)",
            report.checked
        );
        return ExitCode::SUCCESS;
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
    ExitCode::FAILURE
}
