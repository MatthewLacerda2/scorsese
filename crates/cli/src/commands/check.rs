//! `scorsese check`

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{PROJECT_FILE_NAME, Project};
use scorsese_render::unknown_in;

/// Reads the project and reports everything wrong or questionable about it in
/// one pass, without rendering anything.
///
/// Two kinds of finding, and the difference is the point:
///
/// - **Problems** are validation errors — a dangling asset reference, two clips
///   fighting over the same instant. A project with one cannot render, so this
///   exits non-zero.
/// - **Warnings** are things that render perfectly well and are probably not
///   what anyone meant, the keyframe track naming a property nobody animates
///   being the first of them. They never fail anything.
///
/// Both are printed even when there are problems: an agent repairing a project
/// unattended should see the whole list, not discover it one round-trip at a
/// time.
pub fn run(project_dir: &Path) -> Result<()> {
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

    println!(
        "{} — {} asset(s), {} track(s), {} clip(s)",
        project.name,
        project.assets.len(),
        project.tracks.len(),
        project.clips().count()
    );
    match project.validate().err() {
        // Non-zero, and carrying the errors themselves rather than a count:
        // whatever runs this unattended reads the message and nothing else.
        Some(problems) => bail!(problems),
        None if warnings.is_empty() => {
            println!("no problems, nothing to warn about");
            Ok(())
        }
        // Warnings alone are green. They audit quality; they do not prove
        // anything wrong, and a signal that fails a build is a gate.
        None => {
            println!("no problems, {} warning(s)", warnings.len());
            Ok(())
        }
    }
}
