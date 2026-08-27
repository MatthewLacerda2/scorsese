//! Changing one number in a recipe, by the name the document already uses.
//!
//! Writing a recipe and **tuning** one are not the same act. A score is
//! written once and then adjusted a float at a time, many times, with a bake
//! and a measurement between each turn — so replacing the whole document to
//! move one track's `gain` from `0.71` to `0.64` re-sends every note in the
//! piece to say nothing about any of them.
//!
//! This is a **named operation**, not a patch language, and the narrowness is
//! the point. What it can address is what the recipe *names*: the recipe's own
//! top-level numbers, and where a track sits — its `gain` and its `pan`.
//! Tracks have names; notes and arrangement entries do not, and inventing an
//! index for them is how a narrow tool becomes JSON Patch with worse
//! ergonomics — addressing that means something different the moment a note is
//! inserted above it.
//!
//! The result is an ordinary recipe in the format's own canonical form: parsed
//! before it is handed back, still readable, still editable by hand, and still
//! addressed by the hash of its bytes, so a partial edit makes its asset stale
//! by exactly the arithmetic a whole rewrite does.

use scorsese_zimmer::Song;

use super::recipe::{OneShot, Recipe};

/// Every field name [`set`] accepts, for a schema to publish and a client to
/// choose from. Which of them applies depends on the recipe's shape, and a
/// refusal says so.
pub const FIELDS: [&str; 7] = [
    "bpm", "seed", "swing", "gain", "pan", "duration", "velocity",
];

/// The fields a **track** owns rather than the song: where that instrument
/// sits, in level and in position. Both need a `track` and neither means
/// anything without one.
const TRACK_FIELDS: [&str; 2] = ["gain", "pan"];

/// What a song recipe offers, worded for a refusal.
const IN_A_SONG: &str = "bpm, seed, swing, or a track's gain or pan";

/// What a patch recipe offers, worded for a refusal.
const IN_A_PATCH: &str = "duration, velocity, or seed";

/// Where a JSON number stops being able to name a distinct whole one. A seed
/// beyond it would silently become a neighbour of itself.
const SEED_CEILING: f64 = 9_007_199_254_740_992.0;

/// One number to change, and where it lives.
#[derive(Debug, Clone, Copy)]
pub struct Setting<'a> {
    /// The field's name, spelled the way the recipe spells it.
    pub field: &'a str,
    /// The track that owns it, for the fields a track owns. Named, never
    /// numbered — a song's tracks already have names and its notes use them.
    pub track: Option<&'a str>,
    /// What the field becomes.
    pub value: f64,
}

/// A recipe with one number changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The recipe as it now reads, ready to write.
    pub document: String,
    /// What the field said before, written the way the document wrote it — so
    /// a reply can show the move rather than only its destination.
    pub was: String,
    /// `patch` or `song`, for a reply that names what it edited.
    pub kind: &'static str,
}

/// Applies one setting to a recipe document.
///
/// Every refusal happens before anything is produced, so a caller that writes
/// only on `Ok` leaves the file on disk exactly as it was — the contract
/// `project_write` and `synth_write` both keep.
///
/// # Errors
///
/// When the document is not a recipe, when the field is not one this recipe's
/// shape has, when a named track is not in the song, or when the value is not
/// one the field can hold.
pub fn set(json: &str, setting: &Setting) -> Result<Change, String> {
    let mut recipe =
        Recipe::from_json(json).map_err(|problem| format!("it does not parse: {problem}"))?;
    let kind = recipe.kind();
    let was = match &mut recipe {
        Recipe::Song(song) => in_song(song, setting),
        Recipe::Patch(one_shot) => in_patch(one_shot, setting),
    }?;

    let document = recipe
        .to_json()
        .map_err(|problem| format!("the result would not serialise: {problem}"))?;
    // Parsed back before it is returned, and not as a formality: this is the
    // same bargain `synth_write` strikes, held to a document this function
    // wrote rather than one a client did.
    Recipe::from_json(&document)
        .map_err(|problem| format!("the result would not parse: {problem}"))?;
    Ok(Change {
        document,
        was,
        kind,
    })
}

/// The song's own numbers, and the ones its tracks own.
fn in_song(song: &mut Song, setting: &Setting) -> Result<String, String> {
    if TRACK_FIELDS.contains(&setting.field) {
        return on_track(song, setting);
    }
    if let Some(name) = setting.track {
        return Err(format!(
            "`{}` is the song's own field, not track `{name}`'s — leave `track` out",
            setting.field
        ));
    }
    match setting.field {
        "bpm" => real(&mut song.bpm, setting.value),
        "swing" => real(&mut song.swing, setting.value),
        "seed" => whole(&mut song.seed, setting.value),
        other => Err(unknown(other, "song", IN_A_SONG)),
    }
}

/// Where a named track sits in the mix — its level, or its position across the
/// image. These are the numbers this whole operation exists for, because they
/// are the ones a client re-tunes after every listen.
fn on_track(song: &mut Song, setting: &Setting) -> Result<String, String> {
    let field = setting.field;
    let name = setting.track.ok_or_else(|| {
        format!(
            "`{field}` belongs to a track, so `track` is required: the name the \
             song's notes already use"
        )
    })?;
    let named: Vec<&str> = song
        .tracks
        .iter()
        .map(|track| track.name.as_str())
        .collect();
    let known = named.join(", ");
    let track = song
        .tracks
        .iter_mut()
        .find(|track| track.name == name)
        .ok_or_else(|| format!("no track `{name}` in this song — it has: {known}"))?;
    // Out of range is not refused here, because it is not refused anywhere:
    // the renderer clamps a pan to hard over, there being no position past it.
    match field {
        "pan" => real(&mut track.pan, setting.value),
        _ => real(&mut track.gain, setting.value),
    }
}

/// A one-shot's own numbers. It has no tracks, so it has neither `gain` nor
/// `pan`: there is nothing for a fader to balance and nowhere for one voice to
/// sit relative to another.
fn in_patch(one_shot: &mut OneShot, setting: &Setting) -> Result<String, String> {
    if let Some(name) = setting.track {
        return Err(format!(
            "a patch recipe has no tracks, so it has no track `{name}` — `track` \
             is a song's"
        ));
    }
    match setting.field {
        "duration" => real(&mut one_shot.duration, setting.value),
        "velocity" => real(&mut one_shot.velocity, setting.value),
        "seed" => whole(&mut one_shot.seed, setting.value),
        other => Err(unknown(other, "patch", IN_A_PATCH)),
    }
}

/// Replaces a decimal field, answering with what it said before.
fn real(field: &mut f32, value: f64) -> Result<String, String> {
    if !value.is_finite() {
        return Err(format!("`{value}` is not a number a recipe can hold"));
    }
    let was = field.to_string();
    *field = value as f32;
    Ok(was)
}

/// Replaces a whole-number field, refusing anything a seed cannot be.
fn whole(field: &mut u64, value: f64) -> Result<String, String> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > SEED_CEILING {
        return Err(format!(
            "a seed is a whole number and not a negative one — `{value}` is not one"
        ));
    }
    let was = field.to_string();
    *field = value as u64;
    Ok(was)
}

/// A field this recipe's shape does not have, said with what it does have —
/// a refusal that only says no costs the caller another round trip to learn
/// what to ask for instead.
fn unknown(field: &str, kind: &str, takes: &str) -> String {
    format!("a {kind} recipe has no `{field}` to set — it takes {takes}")
}

/// The routing, by the field name rather than through a server.
///
/// `synth_set` is checked end to end over MCP; what is easier to state here is
/// that each name reaches the value it names and that the ones a shape does
/// not have are refused. A field routed to its neighbour writes a legal
/// document that says something the caller did not ask for, which is the
/// failure a "changed nothing on a refusal" test cannot see.
#[cfg(test)]
mod tests {
    use super::*;

    /// A two-track song, spelled out, so a test can say which track moved.
    const SONG: &str = r#"{
        "recipe": "song", "bpm": 96.0, "seed": 1,
        "tracks": [
            { "name": "lead", "gain": 0.9,
              "patch": { "source": { "kind": "noise" },
                         "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } },
            { "name": "bass", "gain": 0.5,
              "patch": { "source": { "kind": "noise" },
                         "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } }
        ],
        "patterns": { "a": { "beats": 4.0, "notes": [
            { "track": "lead", "note": "C3", "start": 0.0, "dur": 1.0 } ] } },
        "arrangement": ["a"]
    }"#;

    /// A one-shot, which has no tracks and therefore neither track field.
    const PATCH: &str = r#"{
        "recipe": "patch", "note": "C4", "duration": 0.4, "velocity": 1.0, "seed": 0,
        "patch": { "source": { "kind": "noise" },
                   "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } }
    }"#;

    fn tune(json: &str, field: &str, track: Option<&str>, value: f64) -> Result<Change, String> {
        set(
            json,
            &Setting {
                field,
                track,
                value,
            },
        )
    }

    /// The new field reaches the new value, on the named track and no other.
    #[test]
    fn pan_places_the_track_it_names() {
        let change = tune(SONG, "pan", Some("bass"), -0.4).expect("a track's pan is settable");
        assert_eq!(change.was, "0", "it was dead centre");
        assert!(
            change.document.contains("\"pan\": -0.4"),
            "{}",
            change.document
        );
        assert!(
            change.document.contains("\"gain\": 0.5"),
            "the fader is untouched"
        );
        let tracks: Vec<&str> = change
            .document
            .match_indices("\"pan\"")
            .map(|(_, s)| s)
            .collect();
        assert_eq!(tracks.len(), 1, "only the named track was placed");
    }

    /// Centre is the default, so setting it back removes the field again —
    /// the recipe's bytes are its cache key, and a tuned-then-untuned song has
    /// to hash to what it always did.
    #[test]
    fn setting_a_pan_back_to_centre_writes_no_pan_at_all() {
        let placed = tune(SONG, "pan", Some("lead"), 0.6).expect("settable");
        let centred = tune(&placed.document, "pan", Some("lead"), 0.0).expect("settable");
        assert!(
            !centred.document.contains("\"pan\""),
            "{}",
            centred.document
        );
    }

    /// Both track fields need a track, and say so rather than guessing one.
    #[test]
    fn a_track_field_without_a_track_is_refused_by_name() {
        for field in TRACK_FIELDS {
            let refusal = tune(SONG, field, None, 0.5).expect_err("it belongs to a track");
            assert!(refusal.contains(field), "the refusal names it: {refusal}");
            assert!(refusal.contains("`track` is required"), "got {refusal}");
        }
    }

    /// A one-shot has no tracks, so it has neither field — and the refusal
    /// says what it does take, which is what saves a round trip.
    #[test]
    fn a_patch_has_neither_of_the_track_fields() {
        for field in TRACK_FIELDS {
            let refusal = tune(PATCH, field, None, 0.5).expect_err("a patch has no tracks");
            assert!(refusal.contains(IN_A_PATCH), "got {refusal}");
        }
    }

    /// The song's own fields still refuse a track, so `pan` joining the track
    /// side did not move `bpm` onto it.
    #[test]
    fn the_songs_own_fields_still_belong_to_the_song() {
        let refusal = tune(SONG, "bpm", Some("lead"), 72.0).expect_err("bpm is the song's");
        assert!(refusal.contains("leave `track` out"), "got {refusal}");
        assert_eq!(
            tune(SONG, "bpm", None, 72.0).expect("settable").was,
            "96",
            "and it still sets without one"
        );
    }
}
