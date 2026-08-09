//! `scorsese check`

pub mod media;

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{
    AssetId, HashCheck, PROJECT_FILE_NAME, Project, ProjectPath, ValidationErrors, asset_status,
};
use scorsese_render::{unknown_fonts, unknown_in};

use media::{Finding, Severity};

/// Reads the project and reports everything wrong or questionable about it in
/// one pass, without rendering anything.
///
/// Two kinds of finding, and the difference is the point:
///
/// - **Problems** make a render impossible — a dangling asset reference, two
///   clips fighting over the same instant, a file a clip references that is
///   not on disk, a `style.font` naming a face this build does not ship. This
///   exits non-zero.
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
pub(crate) fn run(project_dir: &Path, verify: bool) -> Result<()> {
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

    // A warning and never a problem, for the same reason a missing generated
    // file is one: a project that has lost its brief still renders, and the
    // frames it produces are not one pixel different for it. Worth saying
    // loudly all the same — the script is the only part of the edit that
    // cannot be reconstructed by looking at the film.
    let lost_script = missing_script(&project, project_dir);
    if let Some(path) = &lost_script {
        println!("warning: the project's script `{path}` is not there");
    }

    let hashes = if verify {
        HashCheck::Verify
    } else {
        HashCheck::Skip
    };
    // Folded in with the media findings because it is the same kind of answer:
    // a face this build cannot find stops a render exactly as a missing file
    // does, and whoever reads the log should not have to know which half of the
    // command noticed which.
    let mut media = media::findings(&asset_status(&project, project_dir, hashes));
    media.extend(unknown_fonts(&project).into_iter().map(|unknown| Finding {
        asset: AssetId::new(unknown.asset.clone()),
        severity: Severity::Problem,
        detail: unknown.to_string(),
    }));
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
    let warning_count =
        warnings.len() + count(&media, Severity::Warning) + usize::from(lost_script.is_some());
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

/// The script the document names, when there is no file where it says.
///
/// A path that breaks the project-path rules is validation's to report and not
/// this function's, so a badly-shaped one is left alone here rather than
/// producing a second complaint about the same field.
fn missing_script(project: &Project, project_dir: &Path) -> Option<ProjectPath> {
    let script = project.script.clone()?;
    script.check().ok()?;
    (!script.resolve(project_dir).is_file()).then_some(script)
}

fn count(media: &[Finding], severity: Severity) -> usize {
    media
        .iter()
        .filter(|finding| finding.severity == severity)
        .count()
}
