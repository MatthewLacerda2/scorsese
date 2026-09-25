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
//! follows it by construction — which is how a song's tempo map arrived: the
//! [`map`] module, and no caller learning more than that a clock can be built
//! from a song.
//!
//! A song with no tempo map keeps the `f32` arithmetic, in the order the
//! renderer always did it, so neither routing the renderer through here nor
//! adding the map moved a sample of any song that does not write one. A song
//! that does is worked in `f64` by the map, and handed back as `f32` at the
//! edge, where every caller wanted it.

mod map;

use super::Song;
use crate::core::RATE;
use map::Map;

/// The tempo a song is rendered at, as a way to turn beats into seconds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Clock {
    /// The tempo on the first beat — the only one, unless `map` says otherwise.
    bpm: f32,
    /// Seconds per beat, worked out once — the factor every conversion uses
    /// while the tempo is one number.
    beat: f32,
    /// Beats per sample-frame, likewise, for the conversion the other way.
    per_frame: f32,
    /// The tempo map, for a song that writes one. `None` is every song that
    /// does not, and keeps the `f32` arithmetic the renderer always did.
    map: Option<Map>,
}

impl Clock {
    /// A clock at `bpm` beats per minute, all the way through.
    pub(crate) fn at(bpm: f32) -> Self {
        Self {
            bpm,
            beat: 60.0 / bpm,
            per_frame: bpm / (60.0 * RATE),
            map: None,
        }
    }

    /// The clock `song` is written at, over `passes` of its arrangement:
    /// its `bpm`, then its `tempo` changes.
    pub(crate) fn written(song: &Song, passes: u32) -> Self {
        if song.tempo.is_empty() {
            Self::at(song.bpm)
        } else {
            Self::mapped(song, passes, 1.0)
        }
    }

    /// The clock that lands `passes` whole arrangements of `song` exactly on
    /// `seconds`: every tempo in it moved by one factor, so the piece plays
    /// faster or slower without being reshaped.
    ///
    /// A song with one tempo keeps the arithmetic it always had — the tempo
    /// that fits, worked out directly — so a `stretch` fit that moved no
    /// sample before a tempo map existed moves none now.
    pub(crate) fn stretched(song: &Song, passes: u32, seconds: f32) -> Self {
        let beats = song.arrangement_beats();
        if song.tempo.is_empty() {
            return Self::at(beats * passes as f32 * 60.0 / seconds);
        }
        let once = f64::from(Self::written(song, 1).seconds(beats));
        Self::mapped(song, passes, once * f64::from(passes) / f64::from(seconds))
    }

    /// `song`'s tempo map with every tempo in it multiplied by `scale`.
    fn mapped(song: &Song, passes: u32, scale: f64) -> Self {
        let bpm = (f64::from(song.bpm) * scale) as f32;
        Self {
            map: Some(Map::new(
                song.bpm,
                &song.tempo,
                scale,
                song.arrangement_beats(),
                passes,
            )),
            ..Self::at(bpm)
        }
    }

    /// The tempo on the first beat.
    pub(crate) fn bpm(&self) -> f32 {
        self.bpm
    }

    /// How many seconds into the piece `beats` from its start is.
    pub(crate) fn seconds(&self, beats: f32) -> f32 {
        match &self.map {
            None => beats * self.beat,
            Some(map) => map.seconds(f64::from(beats)) as f32,
        }
    }

    /// How long `beats` last when they start `from` beats into the piece —
    /// which, once the tempo moves, depends on where they are played.
    pub(crate) fn span(&self, from: f32, beats: f32) -> f32 {
        match &self.map {
            None => beats * self.beat,
            Some(map) => {
                let from = f64::from(from);
                (map.seconds(from + f64::from(beats)) - map.seconds(from)) as f32
            }
        }
    }

    /// Which beat of the piece sample-frame `frame` falls on — the conversion
    /// the other way round, which is what a curve riding a fader is read at.
    pub(crate) fn beat_at(&self, frame: usize) -> f32 {
        match &self.map {
            None => frame as f32 * self.per_frame,
            Some(map) => map.beats(frame as f64 / f64::from(RATE)) as f32,
        }
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
        assert_eq!(clock.span(3.0, 2.0), 1.0);
        assert_eq!(clock.beat_at(RATE as usize), 2.0);
    }
}
