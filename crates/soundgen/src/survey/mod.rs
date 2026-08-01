//! What a set of recipes is made of, counted.
//!
//! Every other measurement in this crate looks at **one** thing: a bake's
//! level, a section of it, a track inside it. Nothing looks at a *set*, and a
//! set is where a whole class of finding lives — six cues can each come out
//! measuring perfectly and still be one instrument played six times, which is
//! the first thing a listener notices and the one thing no per-bake number can
//! see.
//!
//! It is also the cheapest signal there is. All of it is already written down
//! in the documents: a source kind is a word in a patch, a tempo is a number in
//! a song, a register is the notes themselves. No bake, no samples, no ffmpeg
//! — this is arithmetic over JSON that was parsed anyway.
//!
//! ## It counts, and stops
//!
//! **No score, no grade, no recommendation, and no diversity number.** This
//! reports what the documents say and never what to do about it. Six variations
//! on one instrument is a legitimate thing to write on purpose, and a report
//! that tutted at it would be enforcing a taste as a build step — the same rule
//! [`crate::level`] already holds: a metric treated as an ear produces music
//! that optimises the number and gets worse. A diversity score is precisely the
//! metric that would get optimised, so there is not one.
//!
//! What that leaves is worth having on its own: an assistant that cannot hear
//! *can* count, and "karplus is the loudest track in all six" is a sentence it
//! can produce from the files without anyone listening first.

pub mod pitch;
mod song;

use std::collections::BTreeMap;

pub use pitch::{PitchClasses, Register};

/// One instrument of one song, as its documents state it.
///
/// Everything here is written down rather than measured: the gain is the
/// number in the score, not the level the track came out at. What it came out
/// at is [`crate::level::Layer`], and that costs a bake.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackSurvey {
    /// The track's name in the song.
    pub name: String,
    /// Which generator makes its tone, in the word the recipe spells it with.
    /// Absent when the patch it names could not be resolved — a survey reports
    /// the documents it could read rather than refusing over one it could not.
    pub source: Option<&'static str>,
    /// Its linear gain in the mix, as written.
    pub gain: f32,
    /// The filter's base cutoff in Hz, absent when the patch has no filter.
    /// The envelope and velocity move it from there; this is where it starts.
    pub cutoff: Option<f32>,
}

/// One song: its tempo, its instruments, and the pitch material it is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct SongSurvey {
    /// What to call it in a report — the asset's id, when a project supplied it.
    pub name: String,
    /// The tempo it *plays* at, which is the written one unless `fit` stretched
    /// it. Reporting the written tempo of a stretched song would put every
    /// number in the rollup slightly beside the music.
    pub bpm: f32,
    /// Its instruments, in the order the document lists them.
    pub tracks: Vec<TrackSurvey>,
    /// How high and low its notes reach. Absent when nothing is played.
    pub register: Option<Register>,
    /// Which pitch classes those notes fall in.
    pub pitches: PitchClasses,
}

impl SongSurvey {
    /// The track sitting highest in the mix, by the gain it is written at.
    ///
    /// Gain rather than measured energy, because that is what is on the page
    /// and this whole report costs nothing. Ties go to the first, which is the
    /// order the document lists them in.
    pub fn loudest(&self) -> Option<&TrackSurvey> {
        self.tracks.iter().reduce(|loudest, track| {
            if track.gain > loudest.gain {
                track
            } else {
                loudest
            }
        })
    }
}

/// Every song a project carries, in the order its assets are listed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Survey {
    /// One entry per song recipe.
    pub songs: Vec<SongSurvey>,
}

impl Survey {
    /// The same facts, counted across the set.
    pub fn rollup(&self) -> Rollup {
        let mut sources: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
        let (mut tempo, mut register) = (None::<(f32, f32)>, None::<Register>);
        for song in &self.songs {
            // Once per song, not once per track: a song with three karplus
            // tracks is one song using karplus, and counting tracks would make
            // a thickly-scored piece look like several pieces.
            for kind in song.kinds() {
                sources.entry(kind).or_default().0 += 1;
            }
            if let Some(kind) = song.loudest().and_then(|track| track.source) {
                sources.entry(kind).or_default().1 += 1;
            }
            tempo = Some(match tempo {
                Some((low, high)) => (low.min(song.bpm), high.max(song.bpm)),
                None => (song.bpm, song.bpm),
            });
            register = match (register, song.register) {
                (Some(spread), Some(one)) => Some(spread.union(one)),
                (spread, one) => spread.or(one),
            };
        }
        let mut sources: Vec<SourceCount> = sources
            .into_iter()
            .map(|(source, (songs, loudest))| SourceCount {
                source,
                songs,
                loudest,
            })
            .collect();
        // Most-used first, so the row a reader is looking for is the top one;
        // name breaks a tie, so the order is the same on every run.
        sources.sort_by(|a, b| {
            b.songs
                .cmp(&a.songs)
                .then(b.loudest.cmp(&a.loudest))
                .then(a.source.cmp(b.source))
        });
        Rollup {
            songs: self.songs.len(),
            sources,
            tempo,
            register,
        }
    }
}

/// How often one source kind turns up across the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCount {
    /// The kind, as a recipe spells it.
    pub source: &'static str,
    /// How many songs have at least one track using it.
    pub songs: usize,
    /// How many songs have it in their loudest track — the element a listener
    /// hears first, and the count the per-song rows cannot add up on their own.
    pub loudest: usize,
}

/// The set, counted.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rollup {
    /// How many songs were surveyed.
    pub songs: usize,
    /// One entry per source kind used anywhere, most-used first.
    pub sources: Vec<SourceCount>,
    /// The slowest and fastest tempo. Absent when there are no songs.
    pub tempo: Option<(f32, f32)>,
    /// The lowest and highest note anything plays. Absent when nothing does.
    pub register: Option<Register>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(name: &str, source: &'static str, gain: f32) -> TrackSurvey {
        TrackSurvey {
            name: name.to_owned(),
            source: Some(source),
            gain,
            cutoff: None,
        }
    }

    fn song(name: &str, bpm: f32, tracks: Vec<TrackSurvey>) -> SongSurvey {
        SongSurvey {
            name: name.to_owned(),
            bpm,
            tracks,
            register: Some(Register::at(40.0)),
            pitches: PitchClasses::default(),
        }
    }

    /// The finding the whole report exists for, as a count: one source kind is
    /// the loudest thing in every piece.
    #[test]
    fn the_rollup_counts_which_kind_is_loudest_and_in_how_many() {
        let survey = Survey {
            songs: vec![
                song(
                    "one",
                    96.0,
                    vec![
                        track("arp", "karplus", 0.85),
                        track("pad", "osc_stack", 0.4),
                    ],
                ),
                song(
                    "two",
                    132.0,
                    vec![
                        track("arp", "karplus", 0.76),
                        track("pad", "osc_stack", 0.5),
                    ],
                ),
            ],
        };
        let rollup = survey.rollup();
        assert_eq!(rollup.songs, 2);
        assert_eq!(rollup.sources[0].source, "karplus");
        assert_eq!(rollup.sources[0].songs, 2);
        assert_eq!(rollup.sources[0].loudest, 2);
        assert_eq!(rollup.sources[1].source, "osc_stack");
        assert_eq!(rollup.sources[1].loudest, 0);
        assert_eq!(rollup.tempo, Some((96.0, 132.0)));
    }

    /// A song counts once for a kind however many tracks use it.
    #[test]
    fn one_song_scored_thickly_is_still_one_song() {
        let survey = Survey {
            songs: vec![song(
                "thick",
                100.0,
                vec![
                    track("a", "karplus", 0.5),
                    track("b", "karplus", 0.5),
                    track("c", "karplus", 0.5),
                ],
            )],
        };
        assert_eq!(survey.rollup().sources[0].songs, 1);
    }

    #[test]
    fn nothing_surveyed_counts_nothing() {
        let rollup = Survey::default().rollup();
        assert_eq!(rollup.songs, 0);
        assert!(rollup.sources.is_empty());
        assert_eq!(rollup.tempo, None);
    }
}
