//! A `blend` with nothing beneath it to blend with.
//!
//! `add`, `screen` and `multiply` are about what is already on the canvas, and
//! a clip that is the lowest thing on screen for its whole length has only the
//! empty canvas under it. On the frame itself that canvas is black, and adding
//! or screening light onto black is exactly `normal` — the blend asked for
//! does nothing anybody can see. `multiply` is worse: anything times black is
//! black, so the whole layer disappears. Inside a group the canvas is
//! transparent instead, and every mode over nothing draws exactly as `normal`
//! does.
//!
//! **Always a warning.** The render is correct; it is only probably not what
//! the author meant, which is the one thing a warning is for. And it stays
//! quiet the moment anything at all shares any instant of the clip on a track
//! below it, because whether *that* is enough to blend with is a question
//! about pixels a checkup does not draw.

use scorsese_core::{Blend, Clip, Project, Track, TrackKind};

/// Every clip whose blend has nothing beneath it, already worded.
pub(super) fn over_nothing(project: &Project) -> Vec<String> {
    let mut found = Vec::new();
    lanes(&project.tracks, None, &mut found);
    for asset in &project.assets {
        if let Some(group) = &asset.group {
            lanes(&group.tracks, Some(asset.id.as_str()), &mut found);
        }
    }
    found
}

/// The clips of one stack of tracks — the project's, or one group's — whose
/// blend meets only the empty canvas.
fn lanes(tracks: &[Track], group: Option<&str>, found: &mut Vec<String>) {
    let video: Vec<&Track> = tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video)
        .collect();
    for (at, track) in video.iter().enumerate() {
        for clip in track.clips.iter().filter(|clip| !clip.blend.is_normal()) {
            let beneath = video[..at]
                .iter()
                .flat_map(|track| &track.clips)
                .any(|below| below.overlaps(clip));
            if !beneath {
                found.push(worded(clip, group));
            }
        }
    }
}

/// The warning, with what the blend actually does where it is.
fn worded(clip: &Clip, group: Option<&str>) -> String {
    let mode = clip.blend.as_str();
    let (place, outcome) = match (group, clip.blend) {
        (Some(group), _) => (
            format!(" in group `{group}`"),
            "the group's empty canvas, so it draws exactly as `normal` would",
        ),
        (None, Blend::Multiply) => (
            String::new(),
            "the black frame, and anything multiplied by black is black: the layer disappears",
        ),
        (None, _) => (
            String::new(),
            "the black frame, where adding light to black draws exactly as `normal` would",
        ),
    };
    format!(
        "clip `{}`{place}: `blend: {mode}` has nothing beneath it — it is the lowest thing \
         on screen for its whole length, so it blends with {outcome}",
        clip.id
    )
}
