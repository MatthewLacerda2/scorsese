//! `scorsese check`

pub mod media;

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{HashCheck, PROJECT_FILE_NAME, Project, ValidationErrors, asset_status};
use scorsese_render::unknown_in;

use media::{Finding, Severity};

/// Reads the project and reports everything wrong or questionable about it in
/// one pass, without rendering anything.
///
/// Two kinds of finding, and the difference is the point:
///
/// - **Problems** make a render impossible — a dangling asset reference, two
///   clips fighting over the same instant, a file a clip references that is
///   not on disk. This exits non-zero.
/// - **Warnings** are things that render perfectly well and are probably not
///   what anyone meant: a keyframe track naming a property nobody animates, a
///   file whose content changed since it was imported.
///
/// The document and the media it points at are both in scope. A project whose
/// JSON is flawless still cannot render if the footage was deleted underneath
/// it, and finding that out from the renderer partway through an encode is
/// exactly what running this instead is meant to avoid.
///
/// `verify` re-hashes every file. It is off by default: existence is cheap and
/// always checked, hashing a whole pool is not, and everything hashing can
/// find is a warning that never changes the exit code.
///
/// Both kinds are printed even when there are problems: an agent repairing a
/// project unattended should see the whole list, not discover it one
/// round-trip at a time.
pub fn run(project_dir: &Path, verify: bool) -> Result<()> {
    let file = project_dir.join(PROJECT_FILE_NAME);
    let json = std::fs::read_to_string(&file)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    // Parsed rather than loaded, because `Project::load` validates and returns
    // nothing to warn about when it refuses. Here the two are reported together.
    let project =
        Project::from_json(&json).with_context(|| format!("reading {}", file.display()))?;

    let warnings = unknown_in(&project);
    for warning in &warnings {
        print!(
            "warning: clip `{}`: nothing animates `{}`",
            warning.clip, warning.property
        );
        match warning.did_you_mean {
            Some(known) => println!(" — did you mean `{known}`?"),
            None => println!(),
        }
    }

    let hashes = if verify {
        HashCheck::Verify
    } else {
        HashCheck::Skip
    };
    let media = media::findings(&asset_status(&project, project_dir, hashes));
    for finding in &media {
        println!("{}: {finding}", finding.severity);
    }

    println!(
        "{} — {} asset(s), {} track(s), {} clip(s)",
        project.name,
        project.assets.len(),
        project.tracks.len(),
        project.clips().count()
    );
    if !verify {
        println!("(hashes not checked — pass --verify to re-hash every file)");
    }

    let invalid = project.validate().err();
    let warning_count = warnings.len() + count(&media, Severity::Warning);
    match problem_report(invalid.as_ref(), &media) {
        // Non-zero, and carrying the problems themselves rather than a count:
        // whatever runs this unattended reads the message and nothing else.
        Some(report) => bail!(report),
        None if warning_count == 0 => {
            println!("no problems, nothing to warn about");
            Ok(())
        }
        // Warnings alone are green. They audit quality; they do not prove
        // anything wrong, and a signal that fails a build is a gate.
        None => {
            println!("no problems, {warning_count} warning(s)");
            Ok(())
        }
    }
}

/// The failure message, or `None` when nothing blocks a render.
///
/// Document problems and media problems land in one list under one heading:
/// they are the same answer to the same question, and whoever reads the log
/// should not have to know which half of the command produced which line.
fn problem_report(invalid: Option<&ValidationErrors>, media: &[Finding]) -> Option<String> {
    let mut lines: Vec<String> = invalid
        .map(|errors| errors.as_slice().iter().map(ToString::to_string).collect())
        .unwrap_or_default();
    lines.extend(
        media
            .iter()
            .filter(|finding| finding.severity == Severity::Problem)
            .map(ToString::to_string),
    );
    if lines.is_empty() {
        return None;
    }

    let plural = if lines.len() == 1 {
        "problem"
    } else {
        "problems"
    };
    let mut report = format!("{} {plural} in this project:", lines.len());
    for line in &lines {
        report.push_str("\n  - ");
        report.push_str(line);
    }
    Some(report)
}

fn count(media: &[Finding], severity: Severity) -> usize {
    media
        .iter()
        .filter(|finding| finding.severity == severity)
        .count()
}
