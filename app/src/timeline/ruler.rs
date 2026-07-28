//! The time ruler across the top of the timeline.

use egui::{Align2, Color32, FontId, Painter, Rect, Stroke, Ui};
use scorsese_core::{Fps, Frames};

use super::view::{View, timecode};

/// How tall the ruler strip is.
pub(super) const HEIGHT: f32 = 22.0;

/// The narrowest a label may be spaced before the ruler chooses a coarser
/// interval. Roughly the width of `00:00:00` plus room to breathe — below this
/// the labels touch and the ruler becomes a smear.
const MIN_LABEL_GAP: f32 = 76.0;

/// Tick intervals a person reads without doing arithmetic, in seconds.
///
/// Whole seconds at the finest: this labels a ruler, and a ruler is allowed to
/// be approximate. The exact frame is what the document holds, and what the
/// inspector shows for a clip you actually care about.
const STEPS: &[u64] = &[1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];

/// The coarsest interval that still fits the width, in seconds.
fn step_seconds(view: View, fps: Fps) -> u64 {
    let per_second = view.pixels_per_second(fps);
    STEPS
        .iter()
        .copied()
        .find(|step| *step as f32 * per_second >= MIN_LABEL_GAP)
        // Past the end of the ladder the view is zoomed so far out that even
        // an hour between labels is too close together. An hour is then the
        // honest answer — the alternative is a ruler with no labels at all.
        .unwrap_or_else(|| STEPS.last().copied().unwrap_or(3600))
}

/// Draws the ruler into `rect`, which is the clip area's width.
pub(super) fn draw(ui: &Ui, painter: &Painter, rect: Rect, view: View, fps: Fps) {
    let faint = ui.visuals().weak_text_color();
    let line = ui.visuals().widgets.noninteractive.bg_stroke.color;
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, line),
    );

    let step = step_seconds(view, fps);
    let font = FontId::monospace(10.0);
    for (frame, x) in ticks(rect, view, fps, step) {
        painter.line_segment(
            [
                egui::pos2(x, rect.bottom() - 5.0),
                egui::pos2(x, rect.bottom()),
            ],
            Stroke::new(1.0, line),
        );
        painter.text(
            egui::pos2(x + 3.0, rect.top() + 2.0),
            Align2::LEFT_TOP,
            timecode(frame, fps),
            font.clone(),
            faint,
        );
    }
}

/// Every labelled tick in view, as `(frame, x)`.
///
/// Walks whole steps from zero rather than from the left edge, so a tick sits
/// on 00:00:30 wherever the view happens to be scrolled to — a ruler whose
/// marks depend on the scroll position is a ruler you cannot read a time off.
fn ticks(rect: Rect, view: View, fps: Fps, step: u64) -> Vec<(Frames, f32)> {
    let per_step = fps.frames(step as f64).get().max(1);
    let first = view.first_visible().get() / per_step;
    let mut found = Vec::new();
    for index in first.. {
        let frame = Frames(index * per_step);
        let x = rect.left() + view.offset_of(frame);
        if x > rect.right() {
            break;
        }
        if x >= rect.left() {
            found.push((frame, x));
        }
        // A guard, not a limit: at the coarsest step a pathological view could
        // otherwise walk for a very long time before leaving the rect.
        if found.len() > 200 {
            break;
        }
    }
    found
}

/// The playhead: a line down the whole timeline, with a handle on the ruler.
pub(super) fn playhead(painter: &Painter, area: Rect, x: f32, colour: Color32) {
    if !(area.left()..=area.right()).contains(&x) {
        return;
    }
    painter.line_segment(
        [egui::pos2(x, area.top()), egui::pos2(x, area.bottom())],
        Stroke::new(1.0, colour),
    );
    // A diamond at the top, because a bare line is hard to grab and harder to
    // find again after scrolling.
    let top = egui::pos2(x, area.top() + 5.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            top + egui::vec2(0.0, -5.0),
            top + egui::vec2(5.0, 0.0),
            top + egui::vec2(0.0, 5.0),
            top + egui::vec2(-5.0, 0.0),
        ],
        colour,
        Stroke::NONE,
    ));
}
