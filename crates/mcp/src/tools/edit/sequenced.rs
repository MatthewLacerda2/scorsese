//! `import` with `sequence: true`: each path a folder of frames, each folder
//! one image sequence.
//!
//! Its own file because what comes back is a different report — one asset
//! made of many files, its gaps and its skips — and because the ordinary
//! import's reply is long enough already without a branch for this.

use std::path::Path;

use scorsese_core::{Project, SequenceImport, import_sequence};
use scorsese_render::Ffprobe;

use crate::tools::Reply;

/// Imports every path as a sequence, saves once, and says what came in. One
/// folder that fails does not cost the others, as for an ordinary import.
pub(super) fn import(
    project: &mut Project,
    dir: &Path,
    paths: &[String],
    probe: &Ffprobe,
) -> Result<Reply, String> {
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    let mut added = false;
    for path in paths {
        match import_sequence(project, dir, Path::new(path), probe) {
            Ok(report) => {
                added |= !report.existed;
                lines.extend(said(project, &report));
            }
            Err(error) => failures.push(format!("{path} — failed: {error}")),
        }
    }
    if added {
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
    }
    if lines.is_empty() {
        return Err(format!("{} — nothing was imported", failures.join("; ")));
    }
    lines.extend(failures);
    Ok(lines.join("\n").into())
}

/// The asset a clip shows first, then what to check: gaps, then skips.
fn said(project: &Project, report: &SequenceImport) -> Vec<String> {
    let timing = project
        .asset(&report.id)
        .and_then(|asset| asset.sequence.as_ref())
        .map_or_else(String::new, ToString::to_string);
    let mut lines = vec![if report.existed {
        format!(
            "{} — already in the pool, nothing copied ({timing})",
            report.id
        )
    } else {
        format!(
            "{} — image_sequence: {timing}; stills {} to {} ({} already in the pool)",
            report.id,
            report.stills.first().map_or("", |id| id.as_str()),
            report.stills.last().map_or("", |id| id.as_str()),
            report.reused
        )
    }];
    lines.extend(report.gaps.iter().map(|gap| {
        format!(
            "  gap: {} missing between {} and {}",
            gap.missing, gap.after, gap.before
        )
    }));
    lines.extend(
        report
            .skipped
            .iter()
            .map(|skipped| format!("{} — skipped: {}", skipped.source, skipped.why)),
    );
    lines
}
