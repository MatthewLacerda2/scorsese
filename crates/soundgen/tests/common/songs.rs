//! Song fixtures: the smallest thing that is recognisably a piece of music.

use std::collections::BTreeMap;

use scorsese_soundgen::patch::{Adsr, Patch, Source};
use scorsese_soundgen::song::{Note, PatchRef, Pattern, Pitch, Song, Track};

/// A short noise blip. Cheap to render, and *stochastic*, so it exercises the
/// per-note seeding path rather than a purely deterministic oscillator.
pub(crate) fn blip() -> Patch {
    Patch {
        source: Source::Noise,
        amp: Adsr {
            a: 0.001,
            d: 0.05,
            s: 0.0,
            r: 0.05,
        },
        filter: None,
        lfo: None,
        fx: vec![],
    }
}

/// One note on `track`, by name.
pub(crate) fn note(track: &str, name: &str, start: f32, dur: f32) -> Note {
    Note {
        track: track.to_owned(),
        note: Pitch::Name(name.to_owned()),
        start,
        dur,
        vel: 1.0,
    }
}

/// Two beats, two notes, one track, arranged twice.
pub(crate) fn song() -> Song {
    let mut patterns = BTreeMap::new();
    patterns.insert(
        "verse".to_owned(),
        Pattern {
            beats: 2.0,
            notes: vec![note("bass", "E2", 0.0, 0.5), note("bass", "B2", 1.0, 0.5)],
        },
    );
    Song {
        bpm: 120.0,
        seed: 7,
        tracks: vec![Track {
            name: "bass".to_owned(),
            patch: PatchRef::Inline(Box::new(blip())),
            gain: 0.8,
            fx: vec![],
        }],
        patterns,
        arrangement: vec!["verse".into(), "verse".into()],
        swing: 0.0,
        humanize: None,
        fx: vec![],
        fit: None,
        fade: None,
        tail: None,
    }
}

/// The `verse` pattern, mutably — every song test that changes a note changes
/// that one.
pub(crate) fn verse(song: &mut Song) -> &mut Pattern {
    song.patterns
        .get_mut("verse")
        .expect("the fixture defines `verse`")
}
