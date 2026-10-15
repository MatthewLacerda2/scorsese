//! A test runs or fails; it never skips (`crates/server`'s crate doc).
//!
//! Two rules, both about Rust:
//!
//! - **`skip`**: an `#[ignore]`, or a `#[cfg_attr(…, ignore…)]`, anywhere. A
//!   test that is skipped reports as passing to anyone reading only the
//!   summary. The two the repo has — the golden fixtures off Linux, the app's
//!   watchdog self-test — carry their reason as an allow.
//! - **`database-env`**: a server test reading `DATABASE_URL` from the
//!   environment. Those tests make their own database from
//!   `SCORSESE_TEST_DATABASE_URL` (or a container) and fail without one; one
//!   reading the server's own variable would run against whatever database
//!   the shell happens to point at, or quietly do nothing when it points at
//!   none.

use std::path::Path;

use super::Finding;
use super::allow::{allowed, is_comment};

/// The rule against skipped tests, as an `allow` names it.
pub const SKIP: &str = "skip";

/// The rule against server tests reading the server's database variable.
pub const DATABASE_ENV: &str = "database-env";

/// Whether the attribute opening at `lines[index]` skips a test. A
/// `cfg_attr` may span lines; it is read up to the line closing it.
fn skips_at(lines: &[&str], index: usize) -> bool {
    let line = lines[index].trim_start();
    if line.starts_with("#[ignore") {
        return true;
    }
    if !line.starts_with("#[cfg_attr(") {
        return false;
    }
    let end = lines[index..]
        .iter()
        .position(|line| line.trim_end().ends_with(")]"))
        .map_or(lines.len(), |at| index + at + 1);
    lines[index..end].iter().any(|line| {
        line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|word| word == "ignore")
    })
}

/// Whether `line` reads the server's `DATABASE_URL` from the environment.
fn reads_database_url(line: &str) -> bool {
    let line = line.replace("SCORSESE_TEST_DATABASE_URL", "");
    line.contains("DATABASE_URL")
        && ["var(", "var_os(", "env!("]
            .iter()
            .any(|read| line.contains(read))
}

/// Whether `path` is one of the server's integration tests.
fn is_server_test(path: &Path) -> bool {
    path.starts_with("crates/server/tests")
}

/// Every skipped test, and every server test reading `DATABASE_URL`, in
/// `text` that no allow excuses.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    if path.extension().is_none_or(|ext| ext != "rs") {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut findings = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        if skips_at(&lines, index) && !allowed(&lines, index, SKIP) {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: Some(index + 1),
                rule: SKIP,
                message: "a skipped test reads as a passing one: make it run, or fail".into(),
            });
        }
        if is_server_test(path) && reads_database_url(line) && !allowed(&lines, index, DATABASE_ENV)
        {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: Some(index + 1),
                rule: DATABASE_ENV,
                message: "server tests make their own database from SCORSESE_TEST_DATABASE_URL; \
                          never read the server's DATABASE_URL"
                    .into(),
            });
        }
    }
    findings
}
