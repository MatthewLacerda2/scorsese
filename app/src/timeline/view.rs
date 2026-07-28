//! Where the timeline is looking, and the one place frames become pixels.
//!
//! Every position in the document is a **frame count** on the project's grid.
//! Every position on the screen is a pixel. This is the only module that knows
//! how to get from one to the other, which is what keeps a rounding decision
//! from being made in five places and disagreeing with itself.
//!
//! Seconds appear here too, and nowhere else in the app: the ruler shows a
//! person a time because a person thinks in times, and the document goes on
//! meaning frames. That is the same rule `scorsese-core` already holds — the
//! grid is exact and seconds are for the edges.

use scorsese_core::{Fps, Frames};

/// How far the view may zoom in, in pixels per frame. One frame a hundred
/// pixels wide is already far past where trimming stops being the problem.
const MAX_SCALE: f32 = 100.0;
/// And out. Below this a two-hour film is a few hundred pixels and every clip
/// is a sliver, which is not a view of anything.
const MIN_SCALE: f32 = 0.005;

/// The window onto the timeline: how far in, and how magnified.
#[derive(Debug, Clone, Copy)]
pub(crate) struct View {
    /// Pixels per frame.
    scale: f32,
    /// The frame at the left edge. Fractional so that scrolling and zooming
    /// are smooth rather than stepping a frame at a time.
    left: f32,
}

impl Default for View {
    /// Opens showing a couple of minutes at 30 fps, from the start — enough
    /// that a short film fits without touching anything.
    fn default() -> Self {
        Self {
            scale: 0.25,
            left: 0.0,
        }
    }
}

impl View {
    /// Where `frame` falls, in pixels from the left edge of the clip area.
    pub(crate) fn offset_of(self, frame: Frames) -> f32 {
        (frame.get() as f32 - self.left) * self.scale
    }

    /// How wide a span of `frames` is, in pixels.
    pub(crate) fn width_of(self, frames: Frames) -> f32 {
        frames.get() as f32 * self.scale
    }

    /// How many frames a span of `pixels` covers — the inverse of
    /// [`View::width_of`], and the only honest way to say "eight pixels" in a
    /// document that is measured in frames.
    ///
    /// What snapping needs: a tolerance written in pixels stays the same size
    /// on screen at every magnification, which is the whole point of measuring
    /// it there rather than in frames.
    pub(crate) fn frames_in(self, pixels: f32) -> Frames {
        Frames((pixels / self.scale).max(0.0).round() as u64)
    }

    /// Which frame sits `offset` pixels from the left edge of the clip area.
    ///
    /// Saturating at zero rather than wrapping: a drag that goes past the
    /// start of the timeline means frame zero, and `Frames` cannot hold
    /// anything less.
    pub(crate) fn frame_at(self, offset: f32) -> Frames {
        let frame = self.left + offset / self.scale;
        Frames(frame.max(0.0).round() as u64)
    }

    /// Scrolls by a pixel delta, never past the start.
    pub(crate) fn scroll(&mut self, pixels: f32) {
        self.left = (self.left + pixels / self.scale).max(0.0);
    }

    /// Zooms by a factor, keeping whatever is under `anchor` where it is.
    ///
    /// Anchored rather than centred because zooming is nearly always aimed at
    /// something — the cut you are inspecting is under the pointer, and a
    /// centred zoom would slide it away exactly when you were looking at it.
    pub(crate) fn zoom(&mut self, factor: f32, anchor: f32) {
        let before = self.frame_at(anchor).get() as f32;
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        self.left = (before - anchor / self.scale).max(0.0);
    }

    /// Zooms so `frames` of timeline fills `pixels` of width, from the start.
    ///
    /// What "fit the whole thing on screen" means, and what a freshly opened
    /// project gets: a view of nothing is a worse first impression than a view
    /// of everything.
    pub(crate) fn fit(&mut self, frames: Frames, pixels: f32) {
        if frames.get() == 0 || pixels <= 0.0 {
            return;
        }
        self.scale = (pixels / frames.get() as f32).clamp(MIN_SCALE, MAX_SCALE);
        self.left = 0.0;
    }

    /// Pixels per second at this magnification — what the ruler needs to
    /// choose a tick spacing a person can read.
    pub(crate) fn pixels_per_second(self, fps: Fps) -> f32 {
        self.scale * fps.as_f64() as f32
    }

    /// The first frame in view, for deciding what is worth drawing.
    pub(crate) fn first_visible(self) -> Frames {
        Frames(self.left.max(0.0) as u64)
    }
}

/// A frame count as `HH:MM:SS`, which is how a person reads a time.
///
/// Whole seconds: this labels a ruler, and a ruler crowded with frame counts
/// is harder to read than one that admits it is approximate. The exact frame
/// is what the document holds and what the inspector will show.
pub(crate) fn timecode(frame: Frames, fps: Fps) -> String {
    let total = fps.seconds(frame).max(0.0) as u64;
    let (hours, minutes, seconds) = (total / 3600, (total / 60) % 60, total % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_and_the_pixel_it_lands_on_agree_in_both_directions() {
        let view = View::default();
        for frame in [0_u64, 1, 30, 600, 12_345] {
            let offset = view.offset_of(Frames(frame));
            assert_eq!(
                view.frame_at(offset),
                Frames(frame),
                "frame {frame} did not survive the round trip"
            );
        }
    }

    /// The reason zooming is anchored at all: the cut you are inspecting is
    /// under the pointer, and a centred zoom would slide it away exactly when
    /// you were looking at it.
    #[test]
    fn zooming_keeps_what_is_under_the_anchor_under_the_anchor() {
        let mut view = View::default();
        // Scrolled well in first, so there is room on the left for the view to
        // move into. Against the start of the timeline there is not — see the
        // test below, which is about that case rather than this one.
        view.scroll(20_000.0);
        let anchor = 400.0;
        let held = view.frame_at(anchor);
        for factor in [1.5, 0.5, 4.0, 1.2] {
            view.zoom(factor, anchor);
            let drift = view.frame_at(anchor).get().abs_diff(held.get());
            assert!(drift <= 2, "the anchored frame drifted by {drift} frames");
        }
    }

    /// Zooming out near the start cannot hold the anchor, because holding it
    /// would mean scrolling to before frame zero. It stops at zero instead —
    /// the alternative is a view of negative time, which does not exist.
    #[test]
    fn zooming_out_at_the_start_stops_at_the_start() {
        let mut view = View::default();
        view.zoom(0.01, 400.0);
        assert_eq!(
            view.frame_at(0.0),
            Frames::ZERO,
            "the left edge must stay at the beginning"
        );
    }

    #[test]
    fn the_view_never_scrolls_before_the_start() {
        let mut view = View::default();
        view.scroll(-100_000.0);
        assert_eq!(view.frame_at(0.0), Frames::ZERO);
    }

    #[test]
    fn fitting_puts_the_whole_span_in_the_width() {
        let mut view = View::default();
        view.fit(Frames(1800), 900.0);
        assert_eq!(view.frame_at(0.0), Frames::ZERO);
        assert_eq!(view.frame_at(900.0), Frames(1800));
    }

    /// A span written in pixels has to mean fewer frames the further in you
    /// are — that is what makes a snap tolerance feel the same at every
    /// magnification instead of grabbing half the film when zoomed out.
    #[test]
    fn a_pixel_span_is_worth_fewer_frames_the_further_in_the_view_is() {
        let mut view = View::default();
        view.fit(Frames(1800), 900.0);
        assert_eq!(view.frames_in(10.0), Frames(20));
        view.zoom(4.0, 0.0);
        assert_eq!(view.frames_in(10.0), Frames(5));
        assert_eq!(view.frames_in(0.0), Frames::ZERO);
    }

    #[test]
    fn timecode_reads_the_way_a_clock_does() {
        assert_eq!(timecode(Frames(0), Fps::THIRTY), "00:00:00");
        assert_eq!(timecode(Frames(450), Fps::THIRTY), "00:00:15");
        assert_eq!(timecode(Frames(1800), Fps::THIRTY), "00:01:00");
        assert_eq!(timecode(Frames(108_000), Fps::THIRTY), "01:00:00");
    }
}
