//! The lanes: one per track, and the clips drawn along them.
//!
//! Video tracks are drawn above audio tracks with a divider between them, and
//! that is the model showing through rather than a style choice. Video tracks
//! composite in array order — the first is the bottom layer — while audio
//! tracks all sum and their order means nothing. Two halves that mean
//! different things should not read as one list.

use egui::{Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, Ui, vec2};
use scorsese_core::{Asset, Clip, ClipId, Project, Track, TrackKind};

use super::view::View;

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

/// Everything drawing a lane needs that is the same for every lane.
///
/// Bundled rather than passed one by one: these travel together through every
/// function here, and a signature long enough to need counting is one nobody
/// reads.
pub(super) struct Paint<'a> {
    /// For the theme's colours, which are the only thing read off it.
    pub(super) ui: &'a Ui,
    /// Where the shapes go.
    pub(super) painter: &'a Painter,
    /// The document, for resolving a clip's asset.
    pub(super) project: &'a Project,
    /// How far in and how magnified.
    pub(super) view: View,
    /// Which clip is selected, if any.
    pub(super) selected: Option<ClipId>,
    /// Where the pointer is, for hit-testing.
    pub(super) pointer: Option<egui::Pos2>,
}

/// Draws one lane's background and its clips, returning the clip the pointer
/// is over.
pub(super) fn draw(paint: &Paint<'_>, lane: Rect, track: &Track) -> Option<ClipId> {
    let Paint {
        ui, painter, view, ..
    } = *paint;
    painter.rect_filled(lane, 2.0, ui.visuals().extreme_bg_color);

    let mut hovered = None;
    for clip in &track.clips {
        let rect = clip_rect(lane, clip, view);
        // A clip scrolled off the edge is not drawn at all: at a wide zoom a
        // long project is thousands of clips, and painting the ones nobody can
        // see is work the scrub loop cannot afford.
        if rect.right() < lane.left() || rect.left() > lane.right() {
            continue;
        }
        let asset = paint.project.asset(&clip.asset);
        let is_selected = paint.selected.as_ref() == Some(&clip.id);
        clip_body(ui, painter, rect.intersect(lane), asset, is_selected);
        label(ui, painter, rect.intersect(lane), clip, asset);
        if paint.pointer.is_some_and(|at| rect.contains(at)) {
            hovered = Some(clip.id.clone());
        }
    }
    hovered
}

/// Where a clip sits in its lane.
pub(super) fn clip_rect(lane: Rect, clip: &Clip, view: View) -> Rect {
    let left = lane.left() + view.offset_of(clip.start);
    // At least a hair wide, so a very short clip at a wide zoom is still
    // something you can see and click rather than nothing at all.
    let width = view.width_of(clip.duration).max(2.0);
    Rect::from_min_size(egui::pos2(left, lane.top()), vec2(width, lane.height()))
}

/// The clip's block: filled by what it is, outlined when selected.
fn clip_body(ui: &Ui, painter: &Painter, rect: Rect, asset: Option<&Asset>, selected: bool) {
    let ready = asset.is_some_and(Asset::has_renderable_media);
    let fill = match asset.map(|a| a.kind) {
        _ if !ready => ui.visuals().widgets.inactive.bg_fill,
        Some(kind) if kind.is_audible() => Color32::from_rgb(52, 96, 84),
        Some(_) => Color32::from_rgb(56, 78, 112),
        None => ui.visuals().error_fg_color,
    };
    painter.rect_filled(rect, CornerRadius::same(3), fill);

    // A clip with nothing behind it yet is outlined rather than filled solid:
    // its slug-card nature should be obvious without clicking, because it is
    // the difference between a cut that has been paid for and one that has not.
    if !ready {
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, ui.visuals().weak_text_color()),
            egui::StrokeKind::Inside,
        );
    }
    if selected {
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(2.0, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
}

/// What the clip says on it: the asset, and its state when it has one worth
/// knowing about.
fn label(ui: &Ui, painter: &Painter, rect: Rect, clip: &Clip, asset: Option<&Asset>) {
    // Below this there is no room for text and a clipped glyph reads as a
    // rendering fault rather than as a small clip.
    if rect.width() < 26.0 {
        return;
    }
    let state = asset
        .filter(|asset| !asset.has_renderable_media())
        .and_then(|asset| asset.state)
        .map(|state| format!("  ({state:?})").to_lowercase())
        .unwrap_or_default();

    painter.text(
        rect.left_top() + vec2(5.0, 4.0),
        Align2::LEFT_TOP,
        format!("{}{state}", clip.asset),
        FontId::proportional(11.0),
        ui.visuals().strong_text_color(),
    );
}

/// The track-label gutter, fixed while the clips scroll under it.
///
/// Fixed on purpose: a timeline whose labels scroll away is one you get lost
/// in, and the label is what tells you whether the row you are dragging on is
/// picture or sound.
pub(super) fn gutter(ui: &Ui, painter: &Painter, lane: Rect, track: &Track) {
    let name = track
        .name
        .clone()
        .unwrap_or_else(|| track.id.as_str().to_owned());
    painter.text(
        lane.left_center() + vec2(6.0, 0.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(11.0),
        ui.visuals().text_color(),
    );
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
