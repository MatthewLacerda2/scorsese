//! The staples every score used to rebuild by hand (#1120), and the rule the
//! whole instrument half of the kit keeps: the room belongs to the song.

use super::effects::EFFECTS;
use super::*;
use scorsese_zimmer::patch::Fx;

/// A reverb on a patch is a room per note: an arpeggio of eight steps is
/// eight rooms, stacked under the song's own one. `docs/recipes.md`, *Where
/// an effect goes*, puts it on the song, and the kit has to agree with it.
#[test]
fn no_instrument_brings_its_own_room() {
    for instrument in KIT.iter().filter(|it| !EFFECTS.contains(&it.name)) {
        assert!(
            !instrument
                .patch()
                .fx
                .iter()
                .any(|fx| matches!(fx, Fx::Reverb { .. })),
            "{} carries a reverb — the song's room and its `send` are the place for one",
            instrument.name
        );
    }
}

#[test]
fn each_staple_sits_where_its_name_says() {
    let bands = |name: &str| bands_of(lookup(name).expect(name));
    assert!(bands("sub").0 > 90, "sub: {:?}", bands("sub"));
    assert!(bands("tom").0 > 70, "tom: {:?}", bands("tom"));
    assert!(
        bands("open_hat").2 > 70,
        "open hat: {:?}",
        bands("open_hat")
    );
    assert!(bands("clap").1 > 80, "clap: {:?}", bands("clap"));
    assert!(bands("bell").1 > 70, "bell: {:?}", bands("bell"));
    for name in ["pluck", "lead", "strings"] {
        let (low, mid, _) = bands(name);
        assert!(mid > low, "{name} is a melody, not a bass: {low}/{mid}");
    }
}

/// What makes an open hat open: it is the closed one, left to ring.
#[test]
fn the_open_hat_rings_past_the_closed_one() {
    let samples = |name: &str| {
        let opts = NoteOpts {
            duration: 0.05,
            ..NoteOpts::default()
        };
        bake_named_note(&lookup(name).expect(name).patch(), "C4", &opts)
            .expect(name)
            .wav
            .len()
    };
    assert!(samples("open_hat") > 3 * samples("hat"));
}
