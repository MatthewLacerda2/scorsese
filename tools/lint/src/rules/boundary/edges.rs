//! Which workspace crate may depend on which, and which one alone may hold an
//! HTTP client or talk to Postgres.
//!
//! The crate map in `CLAUDE.md` and each crate's `lib.rs` *Boundary* say it in
//! prose; [`ALLOWED`] says it once more as a table, and every manifest is held
//! to it. A self-dependency (a crate's dev-dependency on itself, to turn on a
//! feature for its own tests) is not an edge.

use std::path::Path;

use super::super::Finding;
use super::super::allow::allowed;
use super::manifest::Manifest;

/// The rule a crate edge breaks, as an `allow` names it.
pub const EDGE: &str = "crate-edge";

/// The rule a misplaced HTTP or Postgres client breaks.
pub const OWNER: &str = "owner";

/// The prefix every crate of this repository's own is named with.
pub const OURS: &str = "scorsese-";

/// Every crate of ours, and the crates of ours it may depend on.
pub const ALLOWED: &[(&str, &[&str])] = &[
    // The model. Everything stands on it, and it stands on nothing of ours.
    ("scorsese-core", &[]),
    // The guides, compiled in: a leaf, so editing one rebuilds only it.
    ("scorsese-guide", &[]),
    // Synthesis. rusty depends on it too, so it may not pull core in.
    ("scorsese-zimmer", &[]),
    // Frames from a project, and nothing that decodes or encodes them.
    ("scorsese-compositor", &["scorsese-core"]),
    // ffmpeg's orchestration; zimmer for reading loudness and songs only.
    (
        "scorsese-render",
        &["scorsese-core", "scorsese-compositor", "scorsese-zimmer"],
    ),
    // Every vendor call, plus local synthesis.
    ("scorsese-providers", &["scorsese-core", "scorsese-zimmer"]),
    // The two headless front doors, over the same library logic.
    (
        "scorsese-cli",
        &[
            "scorsese-core",
            "scorsese-guide",
            "scorsese-providers",
            "scorsese-render",
        ],
    ),
    (
        "scorsese-mcp",
        &[
            "scorsese-core",
            "scorsese-guide",
            "scorsese-providers",
            "scorsese-render",
        ],
    ),
    // The web API: a client of the same crates, and of mcp for its tools.
    (
        "scorsese-server",
        &[
            "scorsese-core",
            "scorsese-mcp",
            "scorsese-providers",
            "scorsese-render",
        ],
    ),
    // Test infrastructure: the golden-render gate.
    ("scorsese-golden", &["scorsese-core", "scorsese-render"]),
    // Run by hand to re-vendor Lucide; it builds without scorsese.
    ("scorsese-icon-vendor", &[]),
    // This gate: its own workspace, so it builds without scorsese.
    ("scorsese-lint", &[]),
    // The desktop app, its own workspace; providers for the quote it shows.
    (
        "scorsese-app",
        &["scorsese-core", "scorsese-providers", "scorsese-render"],
    ),
];

/// Crates of ours that depend on nothing at all, ours or not, and where that
/// is written.
pub const LEAVES: &[(&str, &str)] = &[(
    "scorsese-guide",
    "guide is a leaf: editing a guide rebuilds only it (crates/guide/src/lib.rs, *Boundary*)",
)];

/// Crates that hold a kind of client, and the one crate of ours each kind
/// belongs to, with where that is written.
pub const OWNERS: &[(&[&str], &str, &str)] = &[
    (
        &["ureq", "reqwest", "isahc", "attohttpc", "curl"],
        "scorsese-providers",
        "only providers holds an HTTP client (crates/providers/src/lib.rs, *Boundary*)",
    ),
    (
        &["sqlx", "postgres", "tokio-postgres", "diesel"],
        "scorsese-server",
        "only server talks to Postgres (crates/server/src/lib.rs)",
    ),
];

/// Every edge and owner `manifest` (read from `path`, whose text is `text`)
/// breaks.
pub fn check(path: &Path, text: &str, manifest: &Manifest) -> Vec<Finding> {
    let lines: Vec<&str> = text.lines().collect();
    let finding = |line: usize, rule, message| Finding {
        path: path.to_path_buf(),
        line: Some(line),
        rule,
        message,
    };
    let Some(name) = manifest.name.as_deref() else {
        return Vec::new();
    };
    let Some((_, allowed_deps)) = ALLOWED.iter().find(|(crate_, _)| *crate_ == name) else {
        if !name.starts_with(OURS) {
            return Vec::new();
        }
        return vec![finding(
            1,
            EDGE,
            format!(
                "`{name}` is not in the crate-edge table (tools/lint/src/rules/boundary/edges.rs): \
                 add a row naming what it may depend on, and a line in CLAUDE.md's crate map"
            ),
        )];
    };
    let mut found = Vec::new();
    for dependency in &manifest.dependencies {
        let dep = dependency.name.as_str();
        if dep.starts_with(OURS)
            && dep != name
            && !allowed_deps.contains(&dep)
            && !allowed(&lines, dependency.line - 1, EDGE)
        {
            found.push(finding(
                dependency.line,
                EDGE,
                format!(
                    "`{name}` may not depend on `{dep}` (CLAUDE.md, *Crate map*; the crate's \
                     lib.rs *Boundary*): move what it needs a layer down, or change the \
                     architecture first"
                ),
            ));
        }
        if let Some((_, written)) = LEAVES.iter().find(|(leaf, _)| *leaf == name)
            && !dep.starts_with(OURS)
            && !allowed(&lines, dependency.line - 1, EDGE)
        {
            found.push(finding(
                dependency.line,
                EDGE,
                format!("`{name}` depends on `{dep}`, but {written}"),
            ));
        }
        for (clients, owner, written) in OWNERS {
            if clients.contains(&dep)
                && name != *owner
                && !allowed(&lines, dependency.line - 1, OWNER)
            {
                found.push(finding(
                    dependency.line,
                    OWNER,
                    format!("`{name}` depends on `{dep}`, but {written}: go through `{owner}`"),
                ));
            }
        }
    }
    found
}
