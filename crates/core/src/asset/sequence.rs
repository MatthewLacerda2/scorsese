//! An `image_sequence`: stills played in order, each held for a number of
//! frames, once or round and round.
//!
//! **A picture that carries its own timeline**, arriving from the other
//! direction to an animated gif. A gif's length is measured from the file at
//! render time and never written down (#374); a sequence's is never written
//! down either — it *follows* from what is: how many stills, times how long
//! each is held. There is no duration field to disagree with them.
//!
//! The stills are **assets**, named by id, for the reason a clip names its
//! asset by id: each is a file the project imported, hashed and probed like any
//! other picture, so every mechanism that already looks after a file —
//! import, the missing-file check, relinking, a stored project's library —
//! looks after these too, with nothing added. The cost is a row per still in
//! the assets table, which a four-hundred-photo timelapse pays four hundred
//! times. Paths inside a directory of the sequence's own would have been
//! shorter, and would have needed a second way of being a file in every one
//! of those places.

use serde::{Deserialize, Serialize};

use super::AssetId;
use crate::time::Frames;

/// What an `image_sequence` asset holds: which stills, in what order, for how
/// long each, and whether it starts again.
///
/// Every option a property **type**, never a value: twelve drawings of a spin
/// held two frames and looping, four hundred photographs held one frame each
/// and not, and three drawings of a mouth held four are one mechanism with
/// different numbers in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSequence {
    /// The stills, in the order they are shown, named by asset id. Each must
    /// be an `image` asset; one may appear more than once.
    pub stills: Vec<AssetId>,
    /// How many timeline frames each still stays on screen. One hold for the
    /// whole sequence: a timelapse, a stop-motion take and a rendered frame
    /// directory are every frame the same.
    ///
    /// On the **timeline's** grid, like every other count of frames in the
    /// document, so the same sequence plays at the same pace whatever rate it
    /// is rendered at. A clip's `speed` retimes it like any footage.
    #[serde(default = "one_frame")]
    pub hold: Frames,
    /// Whether the sequence starts again from its first still when it runs
    /// out. When it does not, a clip longer than the sequence holds the last
    /// still — the way every editor holds a picture, and never a hole in the
    /// middle of a timeline.
    #[serde(default, rename = "loop")]
    pub looping: bool,
}

/// The file formats a sequence's stills may be, by extension, lowercased: the
/// ones that hold exactly one picture.
///
/// A gif or an avif can carry an animation, and a still with a timeline in it
/// inside a sequence with one of its own would be two clocks for one picture.
/// Every format a camera, a renderer or an upscaler writes frames in is here.
pub const SEQUENCE_FORMATS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp"];

/// Whether two of [`SEQUENCE_FORMATS`] are one format: one decoder reads both
/// spellings of a jpeg, and both of a tiff.
pub(crate) fn same_format(a: &str, b: &str) -> bool {
    let canonical = |format: &str| match format {
        "jpeg" => "jpg".to_owned(),
        "tiff" => "tif".to_owned(),
        other => other.to_owned(),
    };
    canonical(a) == canonical(b)
}

fn one_frame() -> Frames {
    Frames(1)
}

impl ImageSequence {
    /// A sequence of these stills, each held one frame, played once — the
    /// shape a directory of rendered frames arrives in.
    pub fn new(stills: Vec<AssetId>) -> Self {
        Self {
            stills,
            hold: one_frame(),
            looping: false,
        }
    }

    /// How long one pass through every still lasts, in timeline frames.
    ///
    /// Derived, never stored: it is the stills times the hold, and a number
    /// written beside them could only ever disagree. Not the ceiling of a clip
    /// showing it — see [`crate::Asset::length`] — since a looping sequence
    /// goes on for ever and one that does not holds its last still.
    pub fn length(&self) -> Frames {
        Frames(self.stills.len() as u64 * self.hold.get())
    }

    /// Which still is on screen `position` timeline frames into the sequence,
    /// by its index in [`ImageSequence::stills`].
    ///
    /// Fractional because a clip's speed puts a frame of the timeline between
    /// two of the sequence's, and the still whose hold that instant falls in
    /// is the one shown — rounding down, as a played source is never shown a
    /// frame early. Past the end, a looping sequence has started again and one
    /// that does not loop is still on its last. `None` only for a sequence
    /// with no stills, which validation refuses.
    pub fn still_at(&self, position: f64) -> Option<usize> {
        let count = self.stills.len();
        if count == 0 {
            return None;
        }
        let hold = self.hold.get().max(1) as f64;
        // A whole number of holds, never negative: the instant before a
        // sequence starts shows its first still, as the instant it starts does.
        let step = (position.max(0.0) / hold).floor() as usize;
        Some(if self.looping {
            step % count
        } else {
            step.min(count - 1)
        })
    }
}

/// A sequence's timing in one phrase — how many stills, held how long, and
/// what happens at the end.
///
/// On the type, for [`crate::MediaMetadata`]'s reason: the CLI, the MCP tools
/// and the window all report it, and one fact worded three ways reads as three
/// facts.
impl std::fmt::Display for ImageSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} stills × {} frame(s) = {} frames, {}",
            self.stills.len(),
            self.hold.get(),
            self.length().get(),
            if self.looping {
                "looping"
            } else {
                "once, then holding its last still"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(stills: usize, hold: u64, looping: bool) -> ImageSequence {
        ImageSequence {
            stills: (0..stills).map(|i| AssetId::new(format!("s{i}"))).collect(),
            hold: Frames(hold),
            looping,
        }
    }

    #[test]
    fn each_still_is_held_for_the_hold_and_then_the_next_is_shown() {
        let three = sequence(3, 4, false);
        let shown: Vec<_> = (0..12).map(|f| three.still_at(f as f64)).collect();
        let expected: Vec<_> = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2].map(Some).into();
        assert_eq!(shown, expected);
        assert_eq!(three.length(), Frames(12));
        assert_eq!(
            three.to_string(),
            "3 stills × 4 frame(s) = 12 frames, once, then holding its last still"
        );
    }

    #[test]
    fn past_its_end_a_sequence_loops_or_holds_its_last_still() {
        assert_eq!(sequence(3, 2, true).still_at(6.0), Some(0));
        assert_eq!(sequence(3, 2, true).still_at(9.5), Some(1));
        assert_eq!(sequence(3, 2, false).still_at(6.0), Some(2));
        assert_eq!(sequence(3, 2, false).still_at(600.0), Some(2));
    }

    #[test]
    fn a_fraction_of_a_frame_shows_the_still_whose_hold_it_falls_in() {
        let two = sequence(2, 2, false);
        assert_eq!(two.still_at(1.99), Some(0));
        assert_eq!(two.still_at(2.0), Some(1));
        assert_eq!(two.still_at(-1.0), Some(0));
    }

    #[test]
    fn nothing_is_shown_from_no_stills_and_a_zero_hold_is_one() {
        assert_eq!(sequence(0, 1, true).still_at(3.0), None);
        assert_eq!(sequence(2, 0, false).still_at(1.0), Some(1));
    }

    #[test]
    fn the_hold_defaults_to_a_frame_and_the_loop_to_off() {
        let read: ImageSequence =
            serde_json::from_str(r#"{ "stills": ["a", "b"] }"#).expect("parses");
        assert_eq!(
            read,
            ImageSequence::new(vec![AssetId::new("a"), AssetId::new("b")])
        );
        let looped: ImageSequence =
            serde_json::from_str(r#"{ "stills": ["a"], "hold": 3, "loop": true }"#)
                .expect("parses");
        assert!(looped.looping);
        assert_eq!(looped.hold, Frames(3));
    }
}
