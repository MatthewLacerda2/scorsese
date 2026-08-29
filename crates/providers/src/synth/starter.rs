//! Recipes to start from.
//!
//! A starter **makes a sound on the first bake**. An empty skeleton would be
//! easier to write and worse to receive: the first thing anyone does with a new
//! recipe is render it, listen, and adjust, and a starter that renders silence
//! teaches nothing and costs an iteration to discover.
//!
//! Both are also deliberately *plain* — one instrument, obvious numbers, no
//! stage that is not doing something audible — so the first edit is a change to
//! something legible rather than a hunt through a wall of settings.

use std::collections::BTreeMap;

use scorsese_zimmer::Patch;
use scorsese_zimmer::patch::{Adsr, Filter, FilterKind, Fx, Osc, Slope, Source, Wave};
use scorsese_zimmer::song::{Note, PatchRef, Pattern, Pitch, Song, Track};

use super::recipe::{OneShot, Recipe};

/// Which starter to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Starter {
    /// One instrument, one note: the shape an effect takes.
    Patch,
    /// Four bars of that instrument: the shape a score takes.
    Song,
}

impl Starter {
    pub(crate) fn recipe(self) -> Recipe {
        match self {
            Self::Patch => Recipe::Patch(OneShot {
                note: Pitch::Name("C4".to_owned()),
                duration: 0.4,
                velocity: 1.0,
                seed: 0,
                patch: pluck(),
            }),
            Self::Song => Recipe::Song(four_bars()),
        }
    }
}

/// A plucked string: bright at the attack, dark as it decays, with a little
/// room around it. Recognisably an instrument rather than a test tone, which
/// is what makes the first bake worth listening to.
fn pluck() -> Patch {
    Patch {
        source: Source::OscStack {
            oscs: vec![
                Osc {
                    wave: Wave::Saw,
                    detune_cents: -6.0,
                    gain: 1.0,
                    octave: 0,
                    voices: 1,
                    spread: 12.0,
                },
                Osc {
                    wave: Wave::Triangle,
                    detune_cents: 6.0,
                    gain: 0.5,
                    octave: -1,
                    voices: 1,
                    spread: 12.0,
                },
            ],
        },
        amp: Adsr {
            a: 0.004,
            d: 0.25,
            s: 0.0,
            r: 0.12,
            curve: 0.0,
        },
        filter: Some(Filter {
            kind: FilterKind::Lowpass,
            cutoff: 700.0,
            slope: Slope::Db12,
            resonance: 0.2,
            // The move that makes it a pluck rather than a pad: the cutoff
            // opens on the attack and closes as the note decays. Two and a
            // half octaves takes 700 Hz to just under 4 kHz.
            env_octaves: 2.5,
            vel_octaves: 0.0,
            adsr: Adsr {
                a: 0.001,
                d: 0.18,
                s: 0.0,
                r: 0.1,
                curve: 0.0,
            },
        }),
        pitch_env: None,
        lfo: None,
        fx: vec![Fx::Reverb {
            size: 0.45,
            damp: 0.5,
            mix: 0.18,
        }],
    }
}

/// Two patterns played twice — a phrase and its answer — so the first thing an
/// editor sees is *how repetition is written*, which is the whole point of the
/// tracker shape.
fn four_bars() -> Song {
    let mut patterns = BTreeMap::new();
    patterns.insert("a".to_owned(), phrase(["C3", "E3", "G3", "E3"]));
    patterns.insert("b".to_owned(), phrase(["A2", "C3", "E3", "C3"]));
    Song {
        bpm: 96.0,
        seed: 1,
        key: None,
        tracks: vec![Track {
            name: "lead".to_owned(),
            patch: PatchRef::Inline(Box::new(pluck())),
            gain: 0.9,
            // The one instrument in the piece, so it belongs in the middle —
            // and being the default, it stays out of the file a starter is
            // read as a worked example of.
            pan: 0.0,
            fx: vec![],
        }],
        patterns,
        arrangement: vec!["a".into(), "b".into(), "a".into(), "b".into()],
        swing: 0.0,
        humanize: None,
        fx: vec![],
        automation: vec![],
        fit: None,
        fade: None,
        tail: None,
    }
}

/// Four notes, one per beat, on the `lead` track.
fn phrase(notes: [&str; 4]) -> Pattern {
    Pattern {
        beats: 4.0,
        notes: notes
            .iter()
            .enumerate()
            .map(|(beat, name)| {
                Note {
                    track: "lead".to_owned(),
                    note: Pitch::Name((*name).to_owned()),
                    start: beat as f32,
                    dur: 0.9,
                    vel: 1.0,
                    articulation: None,
                }
                .into()
            })
            .collect(),
    }
}
