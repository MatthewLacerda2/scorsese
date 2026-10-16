//! What a description says in full, assembled once (#1289).
//!
//! `scorsese describe` and the `project_describe` tool are one question asked
//! from two surfaces, and each assembled its answer — the script and notes,
//! the cut, its notes, each instant's layout — on its own. The tool alone
//! named the word each narration is saying at an instant, so the CLI answered
//! less. Both now print this.

use std::path::Path;

use scorsese_core::words::{self, Saying};
use scorsese_core::{Frames, Project};

use super::{Commentary, Description};
use crate::error::RenderError;
use crate::layout::Layout;
use crate::plan::Plan;
use crate::properties::unknown_in;
use crate::report::Note;
use crate::settings::{Budget, Sizing};

/// Why an instant asked about could not be laid out.
#[derive(Debug, thiserror::Error)]
#[error("frame {}: {source}", frame.get())]
pub struct AtError {
    /// The timeline frame asked about.
    pub frame: Frames,
    /// What laying it out ran into.
    pub source: RenderError,
}

/// Everything a description of `plan` says, of `project` at `project_root`:
/// the script and the notes left on its elements, ahead of the cut since they
/// are its context; the cut and a note for whatever a render would note; then
/// for each instant in `at`, in the order asked, where each clip's content
/// lands and which word each narration is saying.
///
/// Where is measured against the size `asked`, or by default the project's
/// own frame at delivery size (#1116, #1278) — the shape the caller will
/// deliver, since a vertical edit measured against a landscape frame wraps
/// its titles wrongly.
pub fn described(
    project: &Project,
    project_root: &Path,
    plan: &Plan<'_>,
    at: &[Frames],
    asked: Option<Sizing>,
) -> Result<String, AtError> {
    let raster = Sizing::resolve(asked, project, Budget::Delivery);
    let commentary = Commentary::of(project);
    let mut out = String::new();
    if !commentary.is_empty() {
        out.push_str(&format!("{commentary}\n\n"));
    }
    out.push_str(&format!("{} — {}\n", project.name, Description::of(plan)));
    // The same notes a render would carry: a cut whose music was silently
    // trimmed should say so where the cut is looked at, not only where it is
    // encoded.
    for note in plan.notes() {
        out.push_str(&format!("  note: {note}\n"));
    }
    for unknown in unknown_in(project) {
        out.push_str(&format!("  note: {}\n", Note::from(unknown)));
    }
    for &frame in at {
        let layout = Layout::at(project, project_root, raster, frame)
            .map_err(|source| AtError { frame, source })?;
        out.push_str(&format!("\n{layout}\n"));
        out.push_str(&saying(project, project_root, frame));
    }
    Ok(out)
}

/// Which word each generated narration playing at `at` is saying, one line a
/// clip, with the name a page reads it by — nothing at all when no narration
/// is playing. Sync checked without listening.
fn saying(project: &Project, project_root: &Path, at: Frames) -> String {
    let seconds = project.timeline_fps.seconds(at);
    words::saying(project, project_root, seconds)
        .into_iter()
        .map(|(clip, saying)| match saying {
            Saying::Word(word) => format!(
                "  {clip} is saying {:?} — scorsese.words[\"{clip}/{}\"], {:.2}s to {:.2}s\n",
                word.text, word.name, word.start, word.end
            ),
            Saying::Between => format!("  {clip} is between words\n"),
            Saying::Untimed => format!(
                "  {clip} has no word timings: not spoken yet, or spoken before they were kept\n"
            ),
        })
        .collect()
}
