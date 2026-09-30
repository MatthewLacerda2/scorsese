//! The two properties only a text layer has: how far it has revealed, and
//! what its figure is.
//!
//! Neither is applied here or by the compositor — a text layer's pixels are
//! drawn by whoever sets its glyphs, before the layer is composited. What this
//! does is turn a keyframe track into the value that drawing needs, and each
//! of them needs something a single eased number is not.

use scorsese_core::{Between, Easing};

use crate::text::Sweep;

/// Where a reveal is: the sweep, linear in time, with the easing handed on to
/// each piece rather than spent on the sweep.
///
/// **`hold` holds the sweep** rather than being handed to the pieces. Every
/// other easing is a curve a piece's entrance can follow; `hold` is "stay where
/// you are until the next keyframe", and a piece following it would never
/// arrive at all. So a held stretch keeps what it had revealed, exactly as a
/// held opacity keeps its opacity.
pub(super) fn sweep(between: Between) -> Sweep {
    if between.easing == Easing::Hold {
        return Sweep {
            at: between.from.clamp(0.0, 1.0),
            easing: Easing::Linear,
            backwards: false,
        };
    }
    Sweep {
        at: between.linear().clamp(0.0, 1.0),
        easing: between.easing,
        backwards: between.to < between.from,
    }
}

/// What a counter shows: the eased value, **kept between the two keyframes it
/// is travelling between**.
///
/// A box that overshoots its mark is motion; a figure that overshoots its mark
/// is a number on screen that nobody wrote — `151 partitions` on the way to
/// 144. So an overshooting easing still shapes *when* the count gets there,
/// arriving early and holding, and never *what* it says.
pub(super) fn count(between: Between) -> f64 {
    let (low, high) = if between.from <= between.to {
        (between.from, between.to)
    } else {
        (between.to, between.from)
    };
    between.eased().clamp(low, high)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(from: f64, to: f64, progress: f64, easing: Easing) -> Between {
        Between {
            from,
            to,
            progress,
            easing,
        }
    }

    #[test]
    fn a_counter_never_shows_a_figure_past_its_keyframes() {
        let landing = at(0.0, 144.0, 0.8, Easing::BackOut);
        assert!(landing.eased() > 144.0, "the curve itself overshoots");
        assert_eq!(count(landing), 144.0);
        assert_eq!(count(at(144.0, 0.0, 0.2, Easing::BackIn)), 144.0);
    }

    #[test]
    fn a_held_reveal_keeps_what_it_had_and_an_exit_runs_backwards() {
        let held = sweep(at(0.4, 1.0, 0.5, Easing::Hold));
        assert_eq!((held.at, held.easing), (0.4, Easing::Linear));
        let leaving = sweep(at(1.0, 0.0, 0.25, Easing::EaseIn));
        assert_eq!(leaving.at, 0.75);
        assert!(leaving.backwards);
    }
}
