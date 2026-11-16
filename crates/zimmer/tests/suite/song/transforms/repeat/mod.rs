//! A layer that repeats until its entry ends (#1793): a two-bar beat under an
//! eight-bar melody, each written once.
//!
//! The claim is this directory's: **a repeating layer must equal writing its
//! passes out by hand** — as one pattern, every pass in the order it is
//! walked, for the samples (each note's noise is keyed by its place in the
//! walk, so this is also the proof that passes are played rather than
//! photocopied); and as consecutive entries that layer the short pattern under
//! cuts of the long one, for everything that reads the notes without playing
//! them.

mod played;
mod read;
mod written;

use crate::common::songs::{note, played, song};
use scorsese_zimmer::Song;
use scorsese_zimmer::song::{ArrangementEntry, Layer, Layers, Pattern, Play};

use super::setup::play;

/// The fixture's two-beat bass `verse` as the groove under an eight-beat
/// `solo` on a second track, and `solo 0`..`solo 3` — the solo cut at every
/// two beats, each cut's notes moved to its own first beat.
pub(super) fn band() -> Song {
    let mut band = song();
    let mut lead = band.tracks[0].clone();
    lead.name = "lead".to_owned();
    band.tracks.push(lead);
    let pitches = ["B4", "D5", "F#5", "A5"];
    let solo: Vec<_> = (0..4)
        .flat_map(|cut| {
            let at = 2.0 * cut as f32;
            [
                note("lead", pitches[cut], at + 0.5, 0.4),
                note("lead", pitches[3 - cut], at + 1.5, 0.4),
            ]
        })
        .collect();
    for cut in 0..4 {
        let notes = solo[2 * cut..2 * cut + 2]
            .iter()
            .cloned()
            .map(|mut it| {
                it.start -= 2.0 * cut as f32;
                it
            })
            .collect();
        band.patterns.insert(
            format!("solo {cut}"),
            Pattern {
                beats: 2.0,
                notes: played(notes),
            },
        );
    }
    band.patterns.insert(
        "solo".to_owned(),
        Pattern {
            beats: 8.0,
            notes: played(solo),
        },
    );
    band
}

pub(super) fn layered(layers: Vec<Layer>) -> ArrangementEntry {
    Layers { layers }.into()
}

/// `pattern`, repeating until its entry ends.
pub(super) fn repeating(pattern: &str) -> Layer {
    Play {
        repeat: true,
        ..play(pattern)
    }
    .into()
}

/// The solo over the groove, written once each.
pub(super) fn written_once() -> Song {
    let mut once = band();
    once.arrangement = vec![layered(vec!["solo".into(), repeating("verse")])];
    once
}

/// The same music as four entries, the groove named in every one.
pub(super) fn written_out() -> Song {
    let mut out = band();
    out.arrangement = (0..4)
        .map(|cut| layered(vec![format!("solo {cut}").into(), "verse".into()]))
        .collect();
    out
}
