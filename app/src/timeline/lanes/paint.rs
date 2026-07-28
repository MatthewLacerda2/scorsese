//! Drawing one lane: its clips, their labels, and the gutter beside them.

use egui::{Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, Ui, vec2};
use scorsese_core::{Asset, AssetId, Clip, ClipId, Project, Track};

use super::super::view::View;

/// Everything drawing a lane needs that is the same for every lane.
///
/// Bundled rather than passed one by one: these travel together through every
/// function here, and a signature long enough to need counting is one nobody
/// reads.
pub(in crate::timeline) struct Paint<'a> {
    /// For the theme's colours, which are the only thing read off it.
    pub(in crate::timeline) ui: &'a Ui,
    /// Where the shapes go.
    pub(in crate::timeline) painter: &'a Painter,
    /// The document, for resolving a clip's asset.
    pub(in crate::timeline) project: &'a Project,
    /// How far in and how magnified.
    pub(in crate::timeline) view: View,
    /// Which clip is selected, if any.
    pub(in crate::timeline) selected: Option<ClipId>,
    /// An asset picked out in the files panel. Its clips are drawn brighter
    /// and everything else is dimmed, which answers "where is this used?"
    /// without anyone having to read ids off a timeline.
    pub(in crate::timeline) highlighted: Option<AssetId>,
    /// Where the pointer is, for hit-testing.
    pub(in crate::timeline) pointer: Option<egui::Pos2>,
}

/// Draws one lane's background and its clips, returning the clip the pointer
/// is over.
pub(in crate::timeline) fn draw(paint: &Paint<'_>, lane: Rect, track: &Track) -> Option<ClipId> {
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
        // Nothing highlighted means everything is at full strength; something
        // highlighted means everything else steps back. Dimming the rest
        // rather than outlining the few keeps the answer readable when an
        // asset is used twenty times.
        let faded = paint
            .highlighted
            .as_ref()
            .is_some_and(|picked| picked != &clip.asset);
        clip_body(ui, painter, rect.intersect(lane), asset, is_selected, faded);
        label(ui, painter, rect.intersect(lane), clip, asset);
        if paint.pointer.is_some_and(|at| rect.contains(at)) {
            hovered = Some(clip.id.clone());
        }
    }
    hovered
}

/// Where a clip sits in its lane.
fn clip_rect(lane: Rect, clip: &Clip, view: View) -> Rect {
    let left = lane.left() + view.offset_of(clip.start);
    // At least a hair wide, so a very short clip at a wide zoom is still
    // something you can see and click rather than nothing at all.
    let width = view.width_of(clip.duration).max(2.0);
    Rect::from_min_size(egui::pos2(left, lane.top()), vec2(width, lane.height()))
}

/// The clip's block: filled by what it is, outlined when selected, stepped
/// back when something else is highlighted.
fn clip_body(
    ui: &Ui,
    painter: &Painter,
    rect: Rect,
    asset: Option<&Asset>,
    selected: bool,
    faded: bool,
) {
    let ready = asset.is_some_and(Asset::has_renderable_media);
    let fill = match asset.map(|a| a.kind) {
        _ if !ready => ui.visuals().widgets.inactive.bg_fill,
        Some(kind) if kind.is_audible() => Color32::from_rgb(52, 96, 84),
        Some(_) => Color32::from_rgb(56, 78, 112),
        None => ui.visuals().error_fg_color,
    };
    let fill = if faded {
        fill.gamma_multiply(0.35)
    } else {
        fill
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
pub(in crate::timeline) fn gutter(ui: &Ui, painter: &Painter, lane: Rect, track: &Track) {
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
