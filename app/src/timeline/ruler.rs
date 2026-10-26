//! The time ruler across the top of the timeline, and the playhead that hangs
//! from it.
//!
//! As the web's (`Ruler.tsx`): a full-height tick at each label and the label
//! after it, every second when there is room for one and every five or ten
//! when not. No minor ticks and no grid through the lanes — a clip covers the
//! lane it sits on, and a line behind it would only show in the gaps.

use egui::{Align2, Painter, Rect, Stroke, pos2};
use scorsese_core::{Fps, Frames};

use super::view::{View, timecode};
use crate::theme::{marks, palette};

/// How tall the ruler strip is: the web's `h-5`.
pub(super) const HEIGHT: f32 = 20.0;

/// How many seconds lie between two labels: one when a second is 40 pixels
/// wide or more, five from 20, ten below that.
fn every(view: View, fps: Fps) -> u64 {
    let per_second = view.pixels_per_second(fps);
    if per_second >= 40.0 {
        1
    } else if per_second >= 20.0 {
        5
    } else {
        10
    }
}

/// How far left of the strip a tick may stand and still have its label seen.
const LABEL_REACH: f32 = 60.0;

/// Draws the ruler into `rect`, which is the clip area's width.
pub(super) fn draw(painter: &Painter, rect: Rect, view: View, fps: Fps) {
    let colours = palette::of(painter.ctx());
    marks::rule(
        painter,
        rect.left_bottom(),
        rect.right_bottom(),
        colours.border,
    );
    for (frame, x) in ticks(rect, view, fps, every(view, fps)) {
        marks::rule(
            painter,
            pos2(x, rect.top()),
            pos2(x, rect.bottom()),
            colours.border,
        );
        painter.text(
            pos2(x + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            timecode(frame, fps),
            marks::caption_font(),
            colours.muted_foreground,
        );
    }
}

/// Every labelled tick in view, a label every `step` seconds.
///
/// Walks whole steps from zero rather than from the left edge, so a tick sits
/// on 0:30 wherever the view happens to be scrolled to — a ruler whose
/// marks depend on the scroll position is a ruler you cannot read a time off.
fn ticks(rect: Rect, view: View, fps: Fps, step: u64) -> Vec<(Frames, f32)> {
    let per_step = fps.frames(step as f64).get().max(1);
    let first = view.first_visible().get() / per_step;
    let mut found = Vec::new();
    for index in first.saturating_sub(1).. {
        let frame = Frames(index * per_step);
        let x = rect.left() + view.offset_of(frame);
        if x > rect.right() {
            break;
        }
        if x >= rect.left() - LABEL_REACH {
            found.push((frame, x));
        }
        // A guard, not a limit: at the coarsest step a pathological view could
        // otherwise walk for a very long time before leaving the rect.
        if found.len() > 800 {
            break;
        }
    }
    found
}

/// The playhead: a 1px line in the playhead colour from the top of the ruler
/// down the whole timeline. Clicking and dragging on the ruler already scrubs,
/// so it carries no handle.
pub(super) fn playhead(painter: &Painter, area: Rect, x: f32) {
    if !(area.left()..=area.right()).contains(&x) {
        return;
    }
    painter.line_segment(
        [pos2(x, area.top()), pos2(x, area.bottom())],
        Stroke::new(1.0, palette::of(painter.ctx()).playhead),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(width: f32) -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(width, HEIGHT))
    }

    /// The web's rule: a label a second where a second is 40 pixels, every
    /// five from 20, every ten below.
    #[test]
    fn labels_thin_out_as_the_view_zooms_out() {
        let fps = Fps::THIRTY;
        let mut view = View::default();
        view.fit(Frames(300), 600.0); // 60 px a second
        assert_eq!(every(view, fps), 1);
        view.fit(Frames(300), 300.0); // 30
        assert_eq!(every(view, fps), 5);
        view.fit(Frames(300), 150.0); // 15
        assert_eq!(every(view, fps), 10);
    }

    /// A second at 30 fps is 30 frames, and every label stands on one.
    #[test]
    fn every_label_stands_on_a_whole_step() {
        let fps = Fps::THIRTY;
        let mut view = View::default();
        view.fit(Frames(420), 1140.0);
        let found = ticks(strip(1140.0), view, fps, 1);
        assert!(found.len() > 10, "a label a second");
        assert!(found.iter().all(|(frame, _)| frame.get() % 30 == 0));
    }

    /// The reason the walk starts from zero: the marks are where the times are,
    /// not where the scroll happens to have stopped.
    #[test]
    fn a_tick_sits_on_a_whole_step_however_the_view_is_scrolled() {
        let fps = Fps::THIRTY;
        for scroll in [0.0, 37.0, 411.5] {
            let mut view = View::default();
            view.scroll(scroll);
            let step = every(view, fps);
            let per_step = fps.frames(step as f64).get();
            for (frame, _) in ticks(strip(900.0), view, fps, step) {
                assert_eq!(frame.get() % per_step, 0, "a label off the step");
            }
        }
    }

    /// A label whose tick has just scrolled off the left is still drawn, so it
    /// does not pop out of existence the moment its line leaves.
    #[test]
    fn a_label_just_off_the_left_edge_is_kept() {
        let fps = Fps::THIRTY;
        let mut view = View::default();
        view.fit(Frames(420), 1140.0);
        view.scroll(40.0);
        let found = ticks(strip(1140.0), view, fps, 1);
        assert!(found.iter().any(|(_, x)| *x < 0.0));
    }
}
