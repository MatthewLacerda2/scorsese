//! The lanes: one per track, and the clips drawn along them.
//!
//! Video tracks are drawn above audio tracks with a divider between them, and
//! that is the model showing through rather than a style choice. Video tracks
//! composite in array order — the first is the bottom layer — while audio
//! tracks all sum and their order means nothing. Two halves that mean
//! different things should not read as one list.

mod paint;

use egui::{Pos2, Rect, vec2};
use scorsese_core::{ClipId, Project, Track, TrackId, TrackKind};

use super::view::View;

pub(super) use paint::{Paint, draw, gutter};

/// How tall one lane is.
pub(super) const LANE: f32 = 30.0;
/// The gap between lanes.
pub(super) const GAP: f32 = 3.0;
/// The gap where video meets audio, wide enough to read as a division.
pub(super) const DIVIDE: f32 = 9.0;

/// Every track in drawing order — video first, then audio — with the lane's
/// top offset from the start of the lane area.
///
/// One place decides the order and the offsets, so the gutter's labels and the
/// clips they name cannot disagree about which row is which.
pub(super) fn laid_out(project: &Project) -> Vec<(&Track, f32)> {
    let mut out = Vec::with_capacity(project.tracks.len());
    let mut y = 0.0;
    let mut audio_started = false;
    for kind in [TrackKind::Video, TrackKind::Audio] {
        for track in project.tracks.iter().filter(|t| t.kind == kind) {
            if kind == TrackKind::Audio && !audio_started {
                audio_started = true;
                if y > 0.0 {
                    y += DIVIDE;
                }
            }
            out.push((track, y));
            y += LANE + GAP;
        }
    }
    out
}

/// Where the line between picture and sound falls, if the project has both.
///
/// `None` when it has only one kind: a divider with nothing on one side of it
/// is a line that means nothing.
pub(super) fn divider(project: &Project) -> Option<f32> {
    let laid = laid_out(project);
    let first_audio = project
        .tracks
        .iter()
        .position(|track| track.kind == TrackKind::Audio)
        .and_then(|_| {
            laid.iter()
                .find(|(track, _)| track.kind == TrackKind::Audio)
        })?;
    // Only if something is above it.
    (first_audio.1 > 0.0).then(|| first_audio.1 - DIVIDE / 2.0)
}

/// How tall the whole lane area is.
pub(super) fn height(project: &Project) -> f32 {
    laid_out(project).last().map_or(0.0, |(_, top)| top + LANE)
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
    use scorsese_core::{Fps, TrackId};

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

    /// Video above audio whatever order the document lists them in. The
    /// document's order means something for video (it is the compositing
    /// order) and nothing for audio, so the drawing must not imply otherwise.
    #[test]
    fn video_is_laid_out_above_audio_however_the_document_orders_them() {
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
        assert_eq!(order, ["v1", "v2", "a1", "a2"]);
    }

    /// Video tracks keep their own order, because for them it *is* meaning:
    /// the first is the bottom layer.
    #[test]
    fn video_tracks_keep_the_order_the_document_gives_them() {
        let project = project(&[("v2", TrackKind::Video), ("v1", TrackKind::Video)]);
        let order: Vec<&str> = laid_out(&project)
            .iter()
            .map(|(track, _)| track.id.as_str())
            .collect();
        assert_eq!(order, ["v2", "v1"], "not sorted — listed");
    }

    #[test]
    fn the_gap_where_sound_begins_is_wider_than_the_gap_between_lanes() {
        let project = project(&[("v1", TrackKind::Video), ("a1", TrackKind::Audio)]);
        let laid = laid_out(&project);
        let gap = laid[1].1 - (laid[0].1 + LANE);
        assert!(gap > GAP, "picture and sound must read as two halves");
    }

    #[test]
    fn a_project_of_one_kind_has_no_divider() {
        assert!(divider(&project(&[("v1", TrackKind::Video)])).is_none());
        assert!(divider(&project(&[("a1", TrackKind::Audio)])).is_none());
        assert!(divider(&project(&[])).is_none());
        assert!(
            divider(&project(&[
                ("v1", TrackKind::Video),
                ("a1", TrackKind::Audio)
            ]))
            .is_some()
        );
    }

    #[test]
    fn an_empty_project_lays_out_to_nothing() {
        assert_eq!(height(&project(&[])), 0.0);
        assert!(laid_out(&project(&[])).is_empty());
    }
}
