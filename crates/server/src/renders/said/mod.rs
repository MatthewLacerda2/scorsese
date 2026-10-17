//! What a finished render tells its owner (#1289).
//!
//! The lines `scorsese render` prints after its file and the stdio tool
//! answers with — `scorsese_render::say::report` — in the same words: how the
//! delivered soundtrack came out, then a note for each thing the render could
//! not do as asked. Two of those notes are the server's to find: why the
//! capture worker left a page as its card ([`scorsese_render::page::reasoned`]),
//! and which synth assets played a bake older than their recipe or their clip, asked
//! of the laid-out folder exactly as the CLI asks it of a local one. A web
//! render once played such a bake and said nothing.

use std::collections::HashMap;
use std::path::Path;

use scorsese_core::Project;
use scorsese_render::RenderReport;
use scorsese_render::say;

/// What `report` says, of `project` laid out at `root`, with `failed` the
/// pages the worker could not capture and why.
pub(super) fn said(
    mut report: RenderReport,
    project: &Project,
    root: &Path,
    failed: &HashMap<String, String>,
) -> Vec<String> {
    report.notes = scorsese_render::page::reasoned(&report.notes, project, failed);
    let stale = scorsese_providers::synth::out_of_date(project, root);
    say::report(&report, &stale)
}

#[cfg(test)]
mod tests;
