//! Round-tripping, tag spelling, and the defaults an omitted field means.

use crate::common::{adsr, osc};
use scorsese_zimmer::Patch;
use scorsese_zimmer::patch::{Filter, FilterKind, Fx, Lfo, LfoTarget, Source, Wave};

/// A patch touching every optional stage — the round-trip has to carry all of
/// them, so the fixture has all of them.
fn full_patch() -> Patch {
    Patch {
        source: Source::OscStack {
            oscs: vec![osc(Wave::Saw, -7.0, 0), osc(Wave::Square, 7.0, -1)],
        },
        amp: adsr(0.005, 0.1, 0.7, 0.3),
        filter: Some(Filter {
            kind: FilterKind::Lowpass,
            cutoff: 1200.0,
            resonance: 0.4,
            env_amount: 2000.0,
            vel_cutoff: 2500.0,
            adsr: adsr(0.01, 0.2, 0.3, 0.2),
        }),
        lfo: Some(Lfo {
            rate: 5.0,
            depth: 0.5,
            target: LfoTarget::Pitch,
        }),
        fx: vec![
            Fx::Delay {
                time: 0.25,
                feedback: 0.35,
                mix: 0.3,
            },
            Fx::Reverb {
                size: 0.6,
                damp: 0.5,
                mix: 0.2,
            },
        ],
    }
}

#[test]
fn a_patch_round_trips_through_json_unchanged() {
    let patch = full_patch();
    let json = patch.to_json().expect("serialise");
    assert_eq!(Patch::from_json(&json).expect("deserialise"), patch);
}

/// The tags are the words a recipe is written in, so they are part of the
/// contract rather than an implementation detail of the derive.
#[test]
fn tags_are_snake_case_and_named_as_documented() {
    let json = serde_json::to_string(&Source::OscStack { oscs: vec![] }).expect("serialise");
    assert!(json.contains(r#""kind":"osc_stack""#), "got {json}");

    let json = serde_json::to_string(&Fx::Delay {
        time: 0.1,
        feedback: 0.2,
        mix: 0.3,
    })
    .expect("serialise");
    assert!(json.contains(r#""fx":"delay""#), "got {json}");
}

#[test]
fn optional_stages_and_osc_fields_have_defaults() {
    let json = r#"{
        "source": { "kind": "osc_stack", "oscs": [ { "wave": "saw" } ] },
        "amp": { "a": 0.01, "d": 0.1, "s": 0.5, "r": 0.2 }
    }"#;
    let patch = Patch::from_json(json).expect("parses without the optional stages");
    assert!(patch.filter.is_none() && patch.lfo.is_none() && patch.fx.is_empty());
    match &patch.source {
        Source::OscStack { oscs } => {
            assert_eq!(oscs[0].gain, 1.0, "gain defaults to unity");
            assert_eq!(oscs[0].detune_cents, 0.0);
            assert_eq!(oscs[0].octave, 0);
        }
        other => panic!("wrong source: {other:?}"),
    }
}

/// The promise that every recipe written before velocity routing existed still
/// bakes the byte-identical file it always did: both new fields are absent from
/// those documents, and absent has to mean *off*. A bake is addressed by the
/// hash of its recipe, so a default that moved the audio would leave
/// `generated/` serving files no recipe describes.
#[test]
fn velocity_routing_is_off_unless_a_recipe_asks_for_it() {
    let json = r#"{ "kind": "lowpass", "cutoff": 800 }"#;
    let filter: Filter = serde_json::from_str(json).expect("parses");
    assert_eq!(filter.vel_cutoff, 0.0, "velocity adds no cutoff by default");

    let json = r#"{ "kind": "fm2", "ratio": 2.0, "index": 3.0 }"#;
    match serde_json::from_str::<Source>(json).expect("parses") {
        Source::Fm2 { vel_index, .. } => assert_eq!(vel_index, 0.0, "nor any FM depth"),
        other => panic!("wrong source: {other:?}"),
    }
}

#[test]
fn karplus_and_fm_carry_their_documented_defaults() {
    let json = r#"{ "kind": "karplus" }"#;
    match serde_json::from_str::<Source>(json).expect("parses") {
        Source::Karplus {
            damping,
            brightness,
        } => {
            assert_eq!(damping, 0.996);
            assert_eq!(brightness, 0.5);
        }
        other => panic!("wrong source: {other:?}"),
    }

    let json = r#"{ "kind": "fm2", "ratio": 2.0, "index": 3.0 }"#;
    match serde_json::from_str::<Source>(json).expect("parses") {
        Source::Fm2 { mod_decay, .. } => assert_eq!(mod_decay, 0.3),
        other => panic!("wrong source: {other:?}"),
    }
}
