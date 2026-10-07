//! What the narration is saying at an instant — for an assistant checking a
//! page's sync without listening to it.

use std::path::Path;

use crate::{AssetKind, ClipId, Project};

use super::{Placed, Words};

/// What one narration clip is saying at an instant.
#[derive(Debug, Clone, PartialEq)]
pub enum Saying {
    /// This word, placed on the timeline.
    Word(Placed),
    /// Nothing: the instant falls between two words, or before the first or
    /// after the last.
    Between,
    /// The line has no word timings — not generated yet, or generated before
    /// they were kept — so what it says when is not known, and not guessed.
    Untimed,
}

/// Every generated narration clip on the project's timeline playing at
/// `seconds`, with what it is saying then.
pub fn saying(project: &Project, project_root: &Path, seconds: f64) -> Vec<(ClipId, Saying)> {
    let fps = project.timeline_fps;
    project
        .clips()
        .map(|(_, clip)| clip)
        .filter(|clip| {
            let start = fps.seconds(clip.start);
            start <= seconds && seconds < start + fps.seconds(clip.duration)
        })
        .filter_map(|clip| {
            let asset = project.asset(&clip.asset)?;
            if asset.kind != AssetKind::GeneratedAudio {
                return None;
            }
            let saying = match Words::of(asset, project_root) {
                None => Saying::Untimed,
                Some(words) => words
                    .placed(clip, fps)
                    .into_iter()
                    .find(|word| word.start <= seconds && seconds < word.end)
                    .map_or(Saying::Between, Saying::Word),
            };
            Some((clip.id.clone(), saying))
        })
        .collect()
}
