//! The LFO as a document: its shapes, its onset, a rate in beats and a second
//! one (#1772) — what is written, what is refused, and what a note does with
//! them. `core::lfo` pins the curves by the number.

use crate::common::songs::song;
use crate::common::{minimal, opts, osc, refusal, render, saw_patch};
use scorsese_zimmer::patch::{Lfo, LfoShape, LfoTarget, Source, Wave};
use scorsese_zimmer::song::{InlineOnly, PatchRef};
use scorsese_zimmer::{Patch, SynthError, render_song};

fn vibrato() -> Lfo {
    Lfo::new(5.0, 0.5, LfoTarget::Pitch)
}

fn with(lfo: Lfo) -> Patch {
    Patch {
        lfo: Some(lfo),
        ..saw_patch()
    }
}

fn on_the_beat() -> Lfo {
    Lfo {
        beats: Some(0.5),
        ..Lfo::new(0.0, 0.8, LfoTarget::Amp)
    }
}

/// A plain LFO is written as it always was; every new field is left out at
/// its default, and so is an absent `lfo2`, so no recipe's bytes move.
#[test]
fn an_lfo_at_its_defaults_writes_only_what_it_always_did() {
    let json = with(vibrato()).to_json().expect("serialise");
    for absent in ["lfo2", "beats", "shape", "delay", "fade"] {
        assert!(!json.contains(absent), "`{absent}` is in {json}");
    }
    let json = with(on_the_beat()).to_json().expect("serialise");
    assert!(!json.contains("rate"), "a beats LFO writes no rate: {json}");
    let back = Patch::from_json(&json).expect("parse");
    assert_eq!(back.lfo, Some(on_the_beat()));
}

#[test]
fn an_lfo_with_two_speeds_or_none_is_refused() {
    let both = Lfo {
        rate: 2.0,
        ..on_the_beat()
    };
    let none = Lfo::new(0.0, 0.5, LfoTarget::Pitch);
    for (lfo, want) in [
        (
            both,
            SynthError::LfoTwoRates {
                rate: 2.0,
                beats: 0.5,
            },
        ),
        (none, SynthError::LfoNoRate),
    ] {
        assert_eq!(with(lfo).validate_in_song(), Err(want));
    }
}

/// A patch played on its own has no tempo, so `beats` is refused there and
/// accepted in a song; a cycle of no beats is refused everywhere.
#[test]
fn beats_need_a_song_and_a_positive_length() {
    let patch = with(on_the_beat());
    assert_eq!(
        refusal(&patch, 60.0, &opts(0.2)),
        SynthError::LfoBeatsWithoutTempo { beats: 0.5 }
    );
    assert_eq!(patch.validate_in_song(), Ok(()));
    for beats in [0.0, -1.0, f32::INFINITY] {
        let lfo = Lfo {
            beats: Some(beats),
            ..on_the_beat()
        };
        let error = with(lfo).validate_in_song().expect_err("refused");
        assert!(matches!(error, SynthError::BadLfoBeats { .. }), "{error:?}");
    }
}

#[test]
fn a_negative_onset_is_refused() {
    for (lfo, field) in [
        (
            Lfo {
                delay: -0.1,
                ..vibrato()
            },
            "delay",
        ),
        (
            Lfo {
                fade: f32::NAN,
                ..vibrato()
            },
            "fade",
        ),
    ] {
        let error = with(lfo).validate().expect_err("refused");
        assert!(
            matches!(error, SynthError::BadLfoOnset { field: f, .. } if f == field),
            "{error:?}"
        );
    }
}

/// The second LFO is held to the same rules as the first, its pulse-width
/// target included.
#[test]
fn the_second_lfo_is_checked_like_the_first() {
    let patch = Patch {
        lfo2: Some(Lfo::new(1.0, 0.2, LfoTarget::PulseWidth)),
        ..saw_patch()
    };
    assert_eq!(patch.validate(), Err(SynthError::NoPulseToModulate));
    let squared = Patch {
        lfo2: patch.lfo2,
        ..minimal(Source::OscStack {
            oscs: vec![osc(Wave::Square, 0.0, 0)],
        })
    };
    assert_eq!(squared.validate(), Ok(()));
}

/// Through its delay a vibrato moves nothing: the note is sample for sample
/// the note with no LFO, and after it the two part.
#[test]
fn a_delayed_vibrato_leaves_the_start_of_the_note_alone() {
    let late = with(Lfo {
        delay: 0.2,
        fade: 0.3,
        depth: 1.0,
        ..vibrato()
    });
    let plain = render(&saw_patch(), 60.0, &opts(0.8));
    let moved = render(&late, 60.0, &opts(0.8));
    let start = (0.2 * 44_100.0) as usize;
    assert_eq!(moved[..start], plain[..start]);
    assert_ne!(moved[start + 10_000..], plain[start + 10_000..]);
}

/// `lfo2` is an LFO like `lfo`: one written there renders what it renders in
/// the first slot.
#[test]
fn the_second_slot_moves_the_note_as_the_first_does() {
    let first = render(&with(vibrato()), 60.0, &opts(0.4));
    let second = Patch {
        lfo2: Some(vibrato()),
        ..saw_patch()
    };
    assert_eq!(render(&second, 60.0, &opts(0.4)), first);
}

/// A song plays a patch whose LFO is timed in beats, and the same song plays
/// it the same way twice.
#[test]
fn a_song_plays_an_lfo_timed_in_beats() {
    let mut song = song();
    let patch = Patch {
        lfo: Some(Lfo {
            shape: LfoShape::Square,
            ..on_the_beat()
        }),
        ..saw_patch()
    };
    song.tracks[0].patch = PatchRef::Inline(Box::new(patch));
    let once = render_song(&song, &InlineOnly).expect("the song renders");
    assert_eq!(once, render_song(&song, &InlineOnly).expect("again"));
}
