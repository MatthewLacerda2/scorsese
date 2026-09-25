//! Where a beat falls in time: the one conversion from a song's beats to
//! seconds of the piece it renders to.
//!
//! Every time in a song is written in **beats**, and seconds only exist once a
//! tempo has been decided — the planned one, which under a `stretch` fit is
//! not the one written down (see [`super::shape::plan`]). Several things need
//! that conversion: the renderer placing a note and holding it for its gate,
//! a window asked for in beats, and the report saying where each section of
//! the arrangement starts and ends.
//!
//! **They go through one type so they cannot disagree.** A section boundary
//! reported from a private `beats × 60 / bpm` is a different number from the
//! one the notes were placed at the moment anything about the tempo stops
//! being a single constant — and a caption put on a boundary that is a frame
//! off the music is exactly the error the report exists to prevent. A tempo
//! that changes over the piece is a change to this file, and every caller
//! follows it by construction.
//!
//! The arithmetic is kept in `f32` and in the order the renderer always did
//! it, so routing the renderer through here moved no sample.

/// The tempo a song is rendered at, as a way to turn beats into seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Clock {
    bpm: f32,
    /// Seconds per beat, worked out once — the factor every conversion uses.
    beat: f32,
}

impl Clock {
    /// A clock at `bpm` beats per minute.
    pub(crate) fn at(bpm: f32) -> Self {
        Self {
            bpm,
            beat: 60.0 / bpm,
        }
    }

    /// How many seconds into the piece `beats` from its start is — and,
    /// because the tempo is one number, how long a span of `beats` lasts.
    pub(crate) fn seconds(self, beats: f32) -> f32 {
        beats * self.beat
    }

    /// How many beats pass per sample at `rate` — the conversion the other way
    /// round, which is what a curve riding a fader is read at.
    pub(crate) fn beats_per_sample(self, rate: f32) -> f32 {
        self.bpm / (60.0 * rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beat_at_120_bpm_is_half_a_second() {
        let clock = Clock::at(120.0);
        assert_eq!(clock.seconds(1.0), 0.5);
        assert_eq!(clock.seconds(8.0), 4.0);
        assert_eq!(clock.beats_per_sample(1_000.0), 0.002);
    }
}
