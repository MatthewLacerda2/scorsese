//! `scorsese check`

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{HashCheck, PROJECT_FILE_NAME, Project};
use scorsese_render::Checkup;
use scorsese_render::checkup::Verdict;

/// Reads the project and reports everything wrong or questionable about it in
/// one pass, without rendering anything.
///
/// What counts as wrong is [`Checkup`]'s to decide and not this command's: the
/// MCP server's `project_check` asks the same question and has to get the same
/// answer, so all this does is print it and turn the verdict into an exit
/// code. Two kinds of finding, and the difference is what the exit code is
/// made of:
///
/// - **Problems** make a render impossible — a dangling asset reference, two
///   clips fighting over the same instant, a file a clip references that is
///   not on disk, a `style.font` naming a face this build does not ship, an
///   `icon` naming a symbol it does not ship either. This exits non-zero.
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

    let hashes = if verify {
        HashCheck::Verify
    } else {
        HashCheck::Skip
    };
    let checkup = Checkup::of(&project, project_dir, hashes);
    for line in checkup.lines() {
        println!("{line}");
    }
    println!("{}", checkup.summary());
    if !verify {
        println!("(hashes not checked — pass --verify to re-hash every file)");
    }

    match checkup.verdict() {
        // Non-zero, and carrying the problems themselves rather than a count:
        // whatever runs this unattended reads the message and nothing else.
        Verdict::Problems(report) => bail!(report),
        // Warnings alone are green. They audit quality; they do not prove
        // anything wrong, and a signal that fails a build is a gate.
        Verdict::Clear(words) => {
            println!("{words}");
            Ok(())
        }
    }
}
