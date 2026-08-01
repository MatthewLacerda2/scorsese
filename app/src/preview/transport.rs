//! The transport: a scrub bar, and the five ways to move the playhead.
//!
//! Jump to start, back one frame, play or pause, forward one frame, jump to
//! end. Nothing invented — this is the row of buttons under every preview
//! anyone has used, and a preview is not the place to be interesting.
//!
//! **A step is one frame.** Whether a cut lands one frame early is exactly the
//! question a step button exists to answer, and it is the same question
//! `scorsese still` answers from the command line.
//!
//! One control here is not about moving: keeping the frame under the playhead.
//! It sits with the others because it is about *this instant* and nothing else,
//! which is what the whole row is about.
//!
//! Nothing here owns the playhead. Every control reports what it means as a
//! [`Command`] and the caller decides, which is what keeps the one position the
//! timeline also draws from acquiring a second owner here.

use egui::{Align, Layout, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use scorsese_core::{Fps, Frames};

use crate::timeline::timecode;

/// How tall the scrub bar's strip is, and how far the knob may travel from the
/// ends of it.
const BAR: f32 = 18.0;
/// The knob's radius.
const KNOB: f32 = 5.0;

/// What a control on the transport meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    /// Put the playhead here — a button, or a drag on the scrub bar.
    Seek(Frames),
    /// Start running, or stop.
    Toggle,
    /// Keep the frame under the playhead: write it out as a picture.
    ///
    /// Under the preview rather than in a menu because that is where every
    /// editor puts it, and because it is about the frame someone is looking at
    /// — the transport is the row that owns *this instant*. It reports itself
    /// like the others and decides nothing: what "keep it" means is
    /// [`super::save`]'s.
    Keep,
}

/// The last frame anything is on, which is what "the end" means to a transport.
///
/// `length` is the frame *past* the last one, so the last one is below it. An
/// edit with nothing in it answers zero, which is where the playhead already
/// is.
pub(super) fn last_frame(length: Frames) -> Frames {
    Frames(length.get().saturating_sub(1))
}

/// Draws the transport and says what was asked of it.
pub(super) fn show(
    ui: &mut Ui,
    fps: Fps,
    at: Frames,
    last: Frames,
    silent: Option<&str>,
) -> Option<Command> {
    let scrubbed = scrubber(ui, at, last).map(Command::Seek);
    let pressed = buttons(ui, fps, at, last);
    // Said quietly, under the transport, because it is not an error — the film
    // still plays. But a preview that is silent for a reason should give the
    // reason, or the reason someone reaches for is "the sound is broken".
    if let Some(why) = silent {
        ui.label(
            egui::RichText::new(format!("no sound — {why}"))
                .weak()
                .small(),
        );
    }
    // The scrub bar wins a tie, because a drag is continuous and a click is
    // not: letting a button press during a drag jump the playhead elsewhere
    // would make the drag stutter.
    scrubbed.or(pressed)
}

/// The bar: a track with a knob on it, dragged or clicked to seek.
///
/// Drawn rather than assembled from a slider, for the same reason the timeline
/// is: a position in time is a picture, and what a slider would add is a
/// numeric readout nobody wants under a preview.
fn scrubber(ui: &mut Ui, at: Frames, last: Frames) -> Option<Frames> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), BAR), Sense::click_and_drag());
    let track = travel(rect);
    let painter = ui.painter();
    let middle = rect.center().y;
    let visuals = ui.visuals();
    let knob = pos2(track.left() + along(at, last) * track.width(), middle);

    painter.line_segment(
        [pos2(track.left(), middle), pos2(track.right(), middle)],
        Stroke::new(2.0, visuals.widgets.inactive.bg_fill),
    );
    painter.line_segment(
        [pos2(track.left(), middle), knob],
        Stroke::new(2.0, visuals.widgets.active.bg_fill),
    );
    painter.circle_filled(knob, KNOB, visuals.strong_text_color());

    // A bar squeezed to nothing has no fraction to read off it, and dividing by
    // its width would put the playhead at a `NaN`.
    if track.width() <= 0.0 {
        return None;
    }
    let pointer = response.interact_pointer_pos()?;
    Some(frame_at((pointer.x - track.left()) / track.width(), last))
}

/// The five buttons, the one that keeps a frame, and the reading of where the
/// playhead is.
fn buttons(ui: &mut Ui, fps: Fps, at: Frames, last: Frames) -> Option<Command> {
    let mut command = None;
    ui.horizontal(|ui| {
        for (label, hint, what) in [
            ("⏮", "To the start", Command::Seek(Frames::ZERO)),
            ("◀|", "Back one frame", Command::Seek(back(at))),
            ("▶", "Play or pause", Command::Toggle),
            ("|▶", "Forward one frame", Command::Seek(forward(at, last))),
            ("⏭", "To the end", Command::Seek(last)),
        ] {
            if ui.button(label).on_hover_text(hint).clicked() {
                command = Some(what);
            }
        }
        // Set apart from the five, because it is the one control here that
        // does not move the playhead. Words rather than a camera glyph: the
        // shipped fonts are the two the compositor carries and a symbol they
        // do not have draws as an empty box.
        ui.separator();
        if ui
            .button("Save frame")
            .on_hover_text("Write this frame out as a PNG, at delivery resolution")
            .clicked()
        {
            command = Some(Command::Keep);
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{}  ·  frame {}", timecode(at, fps), at.get()))
                    .monospace()
                    .weak(),
            );
        });
    });
    command
}

/// One frame back, and never before the first.
fn back(at: Frames) -> Frames {
    Frames(at.get().saturating_sub(1))
}

/// One frame on, and never past the last.
fn forward(at: Frames, last: Frames) -> Frames {
    Frames(at.get().saturating_add(1).min(last.get()))
}

/// Where `at` sits along the bar: zero at the first frame, one at the last.
///
/// An edit of a single frame — or of none — has nowhere to travel, and answers
/// the start rather than dividing by its own length.
fn along(at: Frames, last: Frames) -> f32 {
    if last.get() == 0 {
        return 0.0;
    }
    at.get().min(last.get()) as f32 / last.get() as f32
}

/// Which frame a fraction along the bar means, clamped to both ends so that a
/// drag carried off the side of the window parks rather than wraps.
fn frame_at(fraction: f32, last: Frames) -> Frames {
    Frames((fraction.clamp(0.0, 1.0) * last.get() as f32).round() as u64)
}

/// The strip the knob's centre may travel along — inset by the knob's own
/// radius, so that parking at either end shows a whole knob rather than half of
/// one hanging off the edge.
fn travel(rect: Rect) -> Rect {
    rect.shrink2(vec2(KNOB + 2.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The off-by-one that decides where "the end" is. `length` counts frames;
    /// the last one is the frame below it, and an empty edit has none.
    #[test]
    fn the_end_is_the_last_frame_and_not_the_one_after_it() {
        assert_eq!(last_frame(Frames(20)), Frames(19));
        assert_eq!(last_frame(Frames(1)), Frames::ZERO);
        assert_eq!(last_frame(Frames::ZERO), Frames::ZERO);
    }

    #[test]
    fn a_step_moves_one_frame_and_stops_at_both_ends() {
        let last = Frames(19);
        assert_eq!(forward(Frames(7), last), Frames(8));
        assert_eq!(back(Frames(7)), Frames(6));
        assert_eq!(back(Frames::ZERO), Frames::ZERO, "before the first frame");
        assert_eq!(forward(last, last), last, "past the last frame");
    }

    #[test]
    fn the_knob_sits_at_the_ends_when_the_playhead_does() {
        let last = Frames(19);
        assert_eq!(along(Frames::ZERO, last), 0.0);
        assert_eq!(along(last, last), 1.0);
        assert!((along(Frames(10), last) - 10.0 / 19.0).abs() < 1e-6);
    }

    /// A one-frame edit, and an empty one. Neither has anywhere for the knob to
    /// go, and dividing by the distance it may travel would be dividing by zero.
    #[test]
    fn a_bar_with_nowhere_to_travel_reads_as_the_start() {
        assert_eq!(along(Frames::ZERO, Frames::ZERO), 0.0);
        assert_eq!(along(Frames(5), Frames::ZERO), 0.0);
        assert_eq!(frame_at(0.7, Frames::ZERO), Frames::ZERO);
    }

    #[test]
    fn a_frame_and_the_place_it_sits_on_the_bar_agree_both_ways() {
        let last = Frames(300);
        for frame in [0_u64, 1, 42, 299, 300] {
            assert_eq!(frame_at(along(Frames(frame), last), last), Frames(frame));
        }
    }

    /// A drag carried off the side of the window: the pointer is past the bar,
    /// and the playhead parks at the end rather than wrapping or running on.
    #[test]
    fn a_drag_off_either_end_parks_there() {
        let last = Frames(19);
        assert_eq!(frame_at(-3.0, last), Frames::ZERO);
        assert_eq!(frame_at(4.0, last), last);
    }
}
