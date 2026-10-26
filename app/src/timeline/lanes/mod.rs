//! The lanes: one per track, and the clips drawn along them.
//!
//! Tracks are drawn in the document's order, as the web's timeline does: for
//! picture that order is the compositing order, and for sound it means nothing,
//! and one list in the document's own order is true of both.

mod paint;

use egui::{Pos2, Rect, vec2};
use scorsese_core::{ClipId, Project, Track, TrackId};

use super::view::View;

pub(super) use paint::{Paint, draw, ground, gutter, trash};

/// How tall one lane is: the web's `h-10`.
pub(super) const LANE: f32 = 40.0;

/// Every track in the document's order, with the lane's top offset from the
/// start of the lane area.
///
/// One place decides the order and the offsets, so the gutter's labels and the
/// clips they name cannot disagree about which row is which.
pub(super) fn laid_out(project: &Project) -> Vec<(&Track, f32)> {
    project
        .tracks
        .iter()
        .enumerate()
        .map(|(row, track)| (track, row as f32 * LANE))
        .collect()
}

/// How tall the whole lane area is.
pub(super) fn height(project: &Project) -> f32 {
    project.tracks.len() as f32 * LANE
}

/// One lane's rectangle: `across` gives it its left edge and width, `top` is
/// where the lanes begin, and `offset` is the track's own from [`laid_out`].
///
/// One function rather than the same three additions in the drawing code and
/// again in the hit-testing code — those two disagreeing is how a clip ends up
/// drawn where it cannot be grabbed.
pub(super) fn lane_rect(across: Rect, top: f32, offset: f32) -> Rect {
    Rect::from_min_size(
        egui::pos2(across.left(), top + offset),
        vec2(across.width(), LANE),
    )
}

/// Which track's lane `y` falls in, and where that lane is.
pub(super) fn lane_at(project: &Project, area: Rect, top: f32, y: f32) -> Option<(&Track, Rect)> {
    laid_out(project)
        .into_iter()
        .map(|(track, offset)| (track, lane_rect(area, top, offset)))
        .find(|(_, lane)| lane.y_range().contains(y))
}

/// What the pointer is over: a clip, the track it is on, and the rectangle it
/// was drawn as.
///
/// The rectangle travels with it because grabbing an *edge* is a question about
/// pixels — how close to the end of the block the pointer is — and the answer
/// has to come from the same geometry the drawing used.
pub(super) struct Hit {
    /// Which clip.
    pub(super) clip: ClipId,
    /// The track holding it, which is where a move starts from.
    pub(super) track: TrackId,
    /// Where it sits on screen.
    pub(super) rect: Rect,
}

/// Which clip is under `at`, if any.
///
/// Deliberately separate from drawing: input is settled before anything is
/// painted, so a clip dragged this frame is drawn where the pointer left it
/// rather than a frame behind.
pub(super) fn hit(project: &Project, area: Rect, top: f32, view: View, at: Pos2) -> Option<Hit> {
    let (track, lane) = lane_at(project, area, top, at.y)?;
    track
        .clips
        .iter()
        .map(|clip| (clip, paint::clip_rect(lane, clip, view)))
        .find(|(_, rect)| rect.contains(at))
        .map(|(clip, rect)| Hit {
            clip: clip.id.clone(),
            track: track.id.clone(),
            rect,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{Fps, TrackId, TrackKind};

    fn track(id: &str, kind: TrackKind) -> Track {
        Track {
            id: TrackId::new(id),
            kind,
            name: None,
            note: None,
            clips: vec![],
        }
    }

    fn project(kinds: &[(&str, TrackKind)]) -> Project {
        let mut project = Project::new("t", Fps::THIRTY);
        project.tracks = kinds.iter().map(|(id, kind)| track(id, *kind)).collect();
        project
    }

    /// The document's order, whatever the kinds: for picture it is the
    /// compositing order, and the web lists tracks the same way.
    #[test]
    fn tracks_are_laid_out_in_the_order_the_document_gives_them() {
        let project = project(&[
            ("a1", TrackKind::Audio),
            ("v1", TrackKind::Video),
            ("a2", TrackKind::Audio),
            ("v2", TrackKind::Video),
        ]);
        let order: Vec<&str> = laid_out(&project)
            .iter()
            .map(|(track, _)| track.id.as_str())
            .collect();
        assert_eq!(order, ["a1", "v1", "a2", "v2"]);
    }

    /// Rows touch: no gap between lanes, no wider gap where sound begins.
    #[test]
    fn lanes_stand_flush_one_under_the_other() {
        let project = project(&[("v1", TrackKind::Video), ("a1", TrackKind::Audio)]);
        let laid = laid_out(&project);
        assert_eq!(laid[1].1, laid[0].1 + LANE);
    }

    #[test]
    fn an_empty_project_lays_out_to_nothing() {
        assert_eq!(height(&project(&[])), 0.0);
        assert!(laid_out(&project(&[])).is_empty());
    }
}
