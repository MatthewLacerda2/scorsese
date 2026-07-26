//! Finding out what a render was never told about its media.
//!
//! Whether a video clip contributes sound is a fact about its file, and the
//! assets table is where that fact is supposed to live — `import` probes and
//! records it. But a `project.json` written by hand, which is the case this
//! editor is built for, has no `media` on anything.
//!
//! Every cheap answer to that has a quiet failure mode. Trusting the metadata
//! alone drops the audio of an unprobed asset in silence, which is the exact
//! surprise mixing video audio exists to remove. Decoding and calling failure
//! silence turns a broken file into a successful render. Probing every clip
//! every time is a process per source on a path that already spawns one per
//! layer, to re-learn something the project is meant to remember.
//!
//! So: use what is recorded, fill in what is not, and **say so** when neither
//! works. A probe that fails leaves the asset as it was, and every clip showing
//! it gets a note naming it and why. Silence is never something the renderer
//! decided on its own without mentioning it.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

use scorsese_core::{Asset, AssetId, ProbeMedia, Project, TrackKind};

use crate::report::Note;
use crate::tools::Tools;

use super::Ffprobe;

/// Probes the video assets this project's video clips show and whose metadata
/// is missing, returning the project with what was found filled in.
///
/// Borrowed and untouched when there was nothing to find out, which is the
/// common case for a project that was imported through `scorsese import`.
pub fn fill_media<'a>(
    tools: &Tools,
    project: &'a Project,
    project_root: &Path,
) -> (Cow<'a, Project>, Vec<Note>) {
    let unprobed: Vec<AssetId> = project
        .assets
        .iter()
        .filter(|asset| needs_probing(project, asset))
        .map(|asset| asset.id.clone())
        .collect();
    if unprobed.is_empty() {
        return (Cow::Borrowed(project), Vec::new());
    }

    let probe = Ffprobe::new(tools.clone());
    let mut filled = project.clone();
    let mut failures = BTreeMap::new();

    for id in unprobed {
        let Some(asset) = filled.assets.iter_mut().find(|asset| asset.id == id) else {
            continue;
        };
        let path = asset
            .path
            .as_ref()
            .expect("only assets with a path are collected");
        match probe.probe(&path.resolve(project_root)) {
            Ok(media) => asset.media = Some(media),
            Err(error) => {
                failures.insert(id, error.message);
            }
        }
    }

    let notes = failures
        .iter()
        .flat_map(|(asset, reason)| skipped_clips(project, asset, reason))
        .collect();
    (Cow::Owned(filled), notes)
}

/// True for an asset a video clip shows, that has a file, and that nobody has
/// asked ffprobe about.
///
/// Only the assets a *video* clip shows: an audio clip is mixed whatever its
/// metadata says, because being on an audio track is already the answer to the
/// question this probe asks.
fn needs_probing(project: &Project, asset: &Asset) -> bool {
    asset.media.is_none()
        && asset.path.is_some()
        && !asset.needs_generation()
        && shown_by_a_video_clip(project, &asset.id)
}

fn shown_by_a_video_clip(project: &Project, asset: &AssetId) -> bool {
    project
        .clips()
        .any(|(track, clip)| track.kind == TrackKind::Video && &clip.asset == asset)
}

/// One note per clip whose own sound was left out, naming the clip — which is
/// what an author can act on — and what stopped us finding out.
fn skipped_clips<'a>(
    project: &'a Project,
    asset: &'a AssetId,
    reason: &'a str,
) -> impl Iterator<Item = Note> + 'a {
    project
        .clips()
        .filter(move |(track, clip)| track.kind == TrackKind::Video && &clip.asset == asset)
        .map(move |(_, clip)| Note::ClipAudioSkipped {
            clip: clip.id.to_string(),
            reason: reason.to_owned(),
        })
}
