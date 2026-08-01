//! The plain controls: one value, one field, no cleverness.
//!
//! Each of these reports the new value on the frame it changed and nothing on
//! every other frame, which is what lets the panel treat "the person changed
//! this" as an event without keeping a copy of the clip to compare against.

use egui::{DragValue, RichText, Ui};
use scorsese_core::{Fit, Fps, Frames, Speed};

use super::time;

/// The slowest and fastest a clip may be dragged to here.
///
/// Not a rule of the format — the document takes any positive rate — but a
/// range a hand can actually aim inside. Beyond it the arithmetic still works
/// and the interesting thing to do is type a number, which the field allows.
const RANGE: std::ops::RangeInclusive<f64> = 0.1..=10.0;

/// The three fits, in the order they are offered, each with the sentence that
/// says what it does to a picture that is not the shape of the frame.
///
/// Laid out flat rather than folded into a menu: there are three, they are
/// short, and one click beats two.
const FITS: [(Fit, &str, &str); 3] = [
    (
        Fit::Fit,
        "Fit",
        "The whole picture, in proportion, with what it does not reach left transparent",
    ),
    (
        Fit::Fill,
        "Fill",
        "Covers the frame, in proportion, cropping whatever overflows the edges",
    ),
    (
        Fit::Native,
        "Native",
        "Its own pixel size, centred and unscaled — how something is placed at the size it was made at",
    ),
];

/// One frame count: the exact number to edit, and the time it lands on for
/// somebody who thinks in seconds. `Some` on the frame the value changed.
///
/// The reading beside the field is of the value *in the field*, not of the
/// document — while a drag is in flight those differ by a frame, and two
/// numbers on one row that disagree are worse than one that is a moment early.
pub(super) fn frames_row(ui: &mut Ui, label: &str, value: Frames, fps: Fps) -> Option<Frames> {
    ui.label(label);
    let mut frames = value.get();
    let changed = ui
        .add(DragValue::new(&mut frames).suffix(" f"))
        .on_hover_text("Frames on the project's grid — drag, or click to type")
        .changed();
    let frames = Frames(frames);
    ui.label(RichText::new(time::readable(frames, fps)).weak().small());
    ui.end_row();
    changed.then_some(frames)
}

/// How fast the clip runs, and what that does to its length on the timeline.
/// `Some` on the frame the rate changed.
///
/// The reading beside the field is the **new duration**, because that is the
/// part a person is about to be surprised by: picking 2× here is the editor's
/// 2×, which shortens the clip to cover the same footage rather than leaving it
/// in place showing twice as much. The document keeps the two apart —
/// `Clip::retime` is what this calls — and the panel is where the convenience
/// belongs, so the number that is going to move is shown before it moves.
pub(super) fn speed_row(ui: &mut Ui, value: Speed, duration: Frames, fps: Fps) -> Option<Speed> {
    ui.label("Speed");
    let mut rate = value.get();
    let changed = ui
        .add(
            DragValue::new(&mut rate)
                .speed(0.01)
                .range(RANGE)
                .suffix("×"),
        )
        .on_hover_text("How fast this plays. The sound goes with it, pitch and all")
        .changed();
    let rate = Speed::new(rate);
    ui.label(
        RichText::new(retimed(value, rate, duration, fps))
            .weak()
            .small(),
    );
    ui.end_row();
    changed.then_some(rate)
}

/// The length the clip would take at the rate in the field, said in the same
/// words as every other time in the panel.
fn retimed(was: Speed, now: Speed, duration: Frames, fps: Fps) -> String {
    if !was.is_usable() || !now.is_usable() {
        return String::new();
    }
    let frames = Frames((now.timeline_frames(was.source_frames(duration)).round() as u64).max(1));
    time::readable(frames, fps)
}

/// The fit choice. `Some` when a different one was picked.
pub(super) fn fit_row(ui: &mut Ui, value: Fit) -> Option<Fit> {
    ui.label("Fit");
    let mut chosen = value;
    ui.horizontal(|ui| {
        for (fit, name, hint) in FITS {
            ui.selectable_value(&mut chosen, fit, name)
                .on_hover_text(hint);
        }
    });
    ui.end_row();
    (chosen != value).then_some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every fit the format has, offered once, with no invented fourth — the
    /// panel is a view of the document's choices and not a menu of its own.
    #[test]
    fn every_fit_is_offered_exactly_once() {
        let mut offered: Vec<Fit> = FITS.iter().map(|(fit, _, _)| *fit).collect();
        offered.dedup();
        assert_eq!(offered.len(), 3, "fit, fill, native — each once");
        assert!(
            offered.contains(&Fit::default()),
            "the default has to be one of them"
        );
    }
}
