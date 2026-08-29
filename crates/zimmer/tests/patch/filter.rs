//! The filter stage as a *document*: what a recipe may write in it, and what
//! it is refused for.
//!
//! The rendering is tested next door under `rendering/`. What is here is the
//! written form, because the written form is the contract an author and an
//! agent both hold: a word that used to work and now means something else is
//! the one failure this stage can have that nobody hears until the bake is
//! wrong.

use scorsese_zimmer::patch::{Filter, FilterKind};

/// Both filter spellings are one word, matching the EQ band kinds — #456.
#[test]
fn the_kinds_are_spelled_as_one_word() {
    for (kind, spelling) in [
        (FilterKind::Lowpass, "lowpass"),
        (FilterKind::Highpass, "highpass"),
    ] {
        let json = serde_json::to_string(&kind).expect("serialise");
        assert_eq!(json, format!("\"{spelling}\""));
    }
}

/// The point of renaming the modulation depths rather than redefining them in
/// place: a recipe written against the Hz fields stops rather than renders a
/// filter that never moves.
#[test]
fn the_hz_era_field_names_are_refused_rather_than_ignored() {
    for stale in ["env_amount", "vel_cutoff"] {
        let json = format!(r#"{{ "kind": "lowpass", "cutoff": 800, "{stale}": 3200 }}"#);
        let refused = serde_json::from_str::<Filter>(&json).expect_err(stale);
        let said = refused.to_string();
        assert!(said.contains(stale), "the offending word is named: {said}");
        assert!(
            said.contains("env_octaves") && said.contains("vel_octaves"),
            "and the ones that work are listed: {said}"
        );
    }
}

/// A depth is a ratio now, so it is read against nothing: two octaves is two
/// octaves whether the filter sits at 200 Hz or at 6.8 kHz, which is the whole
/// of #410 in one assertion.
#[test]
fn a_depth_is_read_the_same_way_wherever_the_cutoff_sits() {
    for cutoff in [200.0, 6800.0] {
        let json = format!(r#"{{ "kind": "lowpass", "cutoff": {cutoff}, "env_octaves": 2 }}"#);
        let filter: Filter = serde_json::from_str(&json).expect("parses");
        assert_eq!(filter.env_octaves, 2.0);
    }
}
