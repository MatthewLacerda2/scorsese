//! The time ruler across the top of the timeline, and the playhead that hangs
//! from it.
//!
//! Three densities of mark, which is what makes a ruler readable rather than
//! merely correct. A **labelled** tick every whole number of seconds a person
//! can do arithmetic with. **Minor** ticks subdividing that, so the eye can
//! judge a third of the way between two labels without reading either. And a
//! **grid line** continuing down through the lanes at each label, faint enough
//! to sit under a clip and still be there behind it — which is what lets you
//! see that two cuts on different tracks land on the same beat.

use egui::{Align2, FontId, Painter, Rect, Shape, Stroke, pos2};
use scorsese_core::{Fps, Frames};

use super::view::{View, timecode};
use crate::theme::{marks, palette};

/// How tall the ruler strip is.
pub(super) const HEIGHT: f32 = 26.0;

/// The narrowest a label may be spaced before the ruler chooses a coarser
/// interval. Roughly the width of `00:00:00` plus room to breathe — below this
/// the labels touch and the ruler becomes a smear.
const MIN_LABEL_GAP: f32 = 76.0;

/// How many minor ticks fall between two labelled ones.
///
/// Four, because every interval on the ladder below divides by four into
/// something a person can name — quarter of a second, half of a five-second
/// step, fifteen of a minute. Five would put a tick at 1.2 seconds, which is a
/// mark nobody can use.
const SUBDIVISIONS: u64 = 4;

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
pub(super) fn draw(painter: &Painter, rect: Rect, view: View, fps: Fps) {
    painter.rect_filled(rect, 0.0, palette::INK);
    marks::rule(
        painter,
        rect.left_bottom(),
        rect.right_bottom(),
        palette::EDGE,
    );

    let step = step_seconds(view, fps);
    let font = FontId::monospace(9.5);
    for Tick { frame, x, labelled } in ticks(rect, view, fps, step, SUBDIVISIONS) {
        let (from, colour) = if labelled {
            (9.0, palette::EDGE)
        } else {
            (4.0, palette::FAINT)
        };
        marks::rule(
            painter,
            pos2(x, rect.bottom() - from),
            pos2(x, rect.bottom()),
            colour,
        );
        if labelled {
            painter.text(
                pos2(x + 4.0, rect.top() + 3.0),
                Align2::LEFT_TOP,
                timecode(frame, fps),
                font.clone(),
                palette::DIM,
            );
        }
    }
}

/// The vertical grid, down through the lanes.
///
/// Drawn before the lanes and not after, so a clip covers it. A grid over the
/// clips would be a stripe through every label on the timeline; a grid under
/// them shows in the gaps, which is exactly where somebody is looking when they
/// are judging whether two cuts line up.
pub(super) fn grid(painter: &Painter, area: Rect, view: View, fps: Fps) {
    let step = step_seconds(view, fps);
    for Tick { frame, x, .. } in ticks(area, view, fps, step, 1) {
        // The start of the edit gets a brighter line: it is the one position on
        // this ruler that is not merely a time but an edge of the film.
        let colour = if frame == Frames::ZERO {
            palette::EDGE
        } else {
            palette::RULE
        };
        marks::rule(
            painter,
            pos2(x, area.top() + HEIGHT),
            pos2(x, area.bottom()),
            colour,
        );
    }
}

/// One mark on the ruler.
struct Tick {
    /// The instant it stands on.
    frame: Frames,
    /// Where that falls on screen.
    x: f32,
    /// Whether this is one of the labelled ones, or a subdivision of them.
    labelled: bool,
}

/// Every tick in view, at `divisions` marks per labelled step.
///
/// Walks whole steps from zero rather than from the left edge, so a tick sits
/// on 00:00:30 wherever the view happens to be scrolled to — a ruler whose
/// marks depend on the scroll position is a ruler you cannot read a time off.
///
/// The division is done in the **index** and not in the frame count, which is
/// the whole of why `labelled` is computed here rather than tested for by the
/// caller. A second at 30 fps is 30 frames and does not divide by four: a tick
/// every `30 / 4 = 7` frames lands on a whole second only every *seven* of them,
/// so a ruler fitted to a fourteen-second cut labelled 00:00:00 and 00:00:07 and
/// nothing in between. Multiplying first and dividing after puts every fourth
/// mark exactly on the step, whatever the rate.
fn ticks(rect: Rect, view: View, fps: Fps, step: u64, divisions: u64) -> Vec<Tick> {
    let per_step = fps.frames(step as f64).get().max(1);
    let divisions = divisions.max(1);
    let first = view.first_visible().get() * divisions / per_step;
    let mut found = Vec::new();
    for index in first.. {
        let frame = Frames(index * per_step / divisions);
        let x = rect.left() + view.offset_of(frame);
        if x > rect.right() {
            break;
        }
        if x >= rect.left() {
            found.push(Tick {
                frame,
                x,
                labelled: index % divisions == 0,
            });
        }
        // A guard, not a limit: at the coarsest step a pathological view could
        // otherwise walk for a very long time before leaving the rect.
        if found.len() > 800 {
            break;
        }
    }
    found
}

/// The playhead: a line down the whole timeline, with a handle on the ruler.
///
/// The handle is a shield rather than the diamond it used to be, and the shape
/// is doing a job: a diamond has the same silhouette upside down, so the point
/// that says *this exact column* is as easy to read at the top as at the
/// bottom. A flat top and a point at the bottom can only be read one way.
pub(super) fn playhead(painter: &Painter, area: Rect, x: f32) {
    if !(area.left()..=area.right()).contains(&x) {
        return;
    }
    let top = area.top() + 3.0;
    painter.line_segment(
        [pos2(x, top), pos2(x, area.bottom())],
        Stroke::new(1.0, palette::PLAYHEAD),
    );
    painter.add(Shape::convex_polygon(
        vec![
            pos2(x - 5.0, top),
            pos2(x + 5.0, top),
            pos2(x + 5.0, top + 11.0),
            pos2(x, top + 17.0),
            pos2(x - 5.0, top + 11.0),
        ],
        palette::PLAYHEAD,
        Stroke::NONE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minor ticks land between the labelled ones and never on top of them, and
    /// there are the promised number of gaps between two labels.
    #[test]
    fn a_labelled_step_is_divided_into_the_promised_number_of_gaps() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(900.0, 26.0));
        let mut view = View::default();
        view.fit(Frames(1800), 900.0);
        let fps = Fps::THIRTY;
        let step = step_seconds(view, fps);
        let all = ticks(rect, view, fps, step, SUBDIVISIONS);
        let labels = ticks(rect, view, fps, step, 1);
        assert!(labels.len() > 1, "a fitted minute has more than one label");
        assert_eq!(
            all.len(),
            labels.len() * SUBDIVISIONS as usize - (SUBDIVISIONS as usize - 1),
            "every gap between two labels carries the same number of minor ticks"
        );
        assert_eq!(
            all.iter().filter(|tick| tick.labelled).count(),
            labels.len(),
            "the marks called labelled are exactly the ones a whole step lands on"
        );
    }

    /// The bug this rule exists for. A second at 30 fps does not divide by four,
    /// and dividing the frame count rather than the index put a label every
    /// *seven* seconds on a ruler that was asked for one every second.
    #[test]
    fn a_step_that_does_not_divide_evenly_still_labels_every_step() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(1140.0, 26.0));
        let fps = Fps::THIRTY;
        let mut view = View::default();
        // Fourteen seconds across the width, which is the fixture the snapshots
        // draw: about 81 pixels a second, so the ruler asks for a label a second.
        view.fit(Frames(420), 1140.0);
        assert_eq!(step_seconds(view, fps), 1);
        let labelled: Vec<u64> = ticks(rect, view, fps, 1, SUBDIVISIONS)
            .iter()
            .filter(|tick| tick.labelled)
            .map(|tick| tick.frame.get())
            .collect();
        assert!(labelled.len() > 10, "a label a second, not one every seven");
        assert!(
            labelled.iter().all(|frame| frame % 30 == 0),
            "every label stands on a whole second: {labelled:?}"
        );
    }

    /// The reason the walk starts from zero: the marks are where the times are,
    /// not where the scroll happens to have stopped.
    #[test]
    fn a_tick_sits_on_a_whole_step_however_the_view_is_scrolled() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(900.0, 26.0));
        let fps = Fps::THIRTY;
        for scroll in [0.0, 37.0, 411.5] {
            let mut view = View::default();
            view.scroll(scroll);
            let step = step_seconds(view, fps);
            let per_step = fps.frames(step as f64).get();
            for tick in ticks(rect, view, fps, step, 1) {
                assert_eq!(
                    tick.frame.get() % per_step,
                    0,
                    "a label off the step ladder"
                );
            }
        }
    }
}
