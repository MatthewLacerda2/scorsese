//! Dissolving one shot into the next: what it writes, and what it moves.

mod placing;
mod writing;

use scorsese_compositor::path;
use scorsese_core::{AssetId, Clip, ClipId, Fps, Frames, Project, Track, TrackId, TrackKind};

/// Two shots meeting at frame 30 on one track, which is the case the whole
/// operation exists for: a cut with nowhere to put a crossover.
pub(crate) fn cut() -> Project {
    let mut project = Project::new("cut", Fps::THIRTY);
    project.tracks.push(Track {
        note: None,
        id: TrackId::new("v1"),
        kind: TrackKind::Video,
        name: None,
        clips: vec![
            Clip::new(
                ClipId::new("a"),
                AssetId::new("shot"),
                Frames(0),
                Frames(30),
            ),
            Clip::new(
                ClipId::new("b"),
                AssetId::new("shot"),
                Frames(30),
                Frames(30),
            ),
        ],
    });
    project
}

/// The clip with this id, wherever it ended up.
pub(crate) fn clip<'a>(project: &'a Project, id: &str) -> &'a Clip {
    project
        .clips()
        .map(|(_, clip)| clip)
        .find(|clip| clip.id.as_str() == id)
        .expect("the clip is somewhere in the project")
}

/// Its opacity keyframes as `(frame, value)`, in order.
pub(crate) fn opacity(project: &Project, id: &str) -> Vec<(u64, f64)> {
    clip(project, id)
        .keyframes
        .iter()
        .filter(|track| track.property.as_str() == path::OPACITY)
        .flat_map(|track| track.keyframes.iter())
        .map(|frame| (frame.t.get(), frame.value))
        .collect()
}

/// Which track in the list holds this clip.
pub(crate) fn track_of(project: &Project, id: &str) -> usize {
    project
        .tracks
        .iter()
        .position(|track| track.clips.iter().any(|clip| clip.id.as_str() == id))
        .expect("the clip is on some track")
}
