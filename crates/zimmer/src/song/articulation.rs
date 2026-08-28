//! How a note is played, as opposed to which note it is.
//!
//! A [`Note`](super::Note) says pitch, onset, length and force. That is the
//! whole of *what* is played and none of *how*, and the gap is most audible in
//! the place a score most often sounds sequenced. A bassline is the clear case:
//! what makes one sound played rather than programmed is almost entirely
//! articulation — a ghost note between the real ones, a couple of accents
//! carrying the groove, a staccato stop where the phrase breathes.
//!
//! **This is not the same thing as [`feel`](super::feel), and the difference is
//! the reason both exist.** Swing and humanise scatter a performance the player
//! did not intend — where the beat is, and how well he hit it. An articulation
//! is what he *did* intend, written on the page. They compose rather than
//! overlap, and the order they compose in is stated in
//! [`render`](super::render).
//!
//! ## Named, never numbers
//!
//! Each of these is a combination of things the engine already does: velocity,
//! gate length, the velocity-to-brightness routings a patch may carry
//! (`vel_cutoff`, `vel_index`), and where the note sits against the beat. A
//! document could write those numbers itself — and then nothing would record
//! that they were an *accent*, every accent in the piece would be a slightly
//! different one, and changing what an accent means would be an edit to every
//! note carrying one. A name says what was meant, and the meaning lives in one
//! place: the table of constants below.
//!
//! ## The set is closed, and it is three
//!
//! `accent`, `staccato`, `ghost`. The test for a fourth is the one
//! `docs/recipes.md` already applies to inversion and retrograde: whether a
//! person reaches for it by hand. **Glide** — a portamento up into a note from
//! the pitch of the one before it — passes that test easily and is deliberately
//! not here: it is the only one of the four with real DSP behind it, and it
//! needs the previous note on its track, which the renderer's *notes are
//! independent* claim does not currently allow. That is a claim to withdraw
//! deliberately rather than as a side effect, so it is #448 and its own
//! argument.
//!
//! ## One articulation, not a set of them
//!
//! An entry carries at most one, so `["accent", "staccato"]` is not writable.
//! Each name here is a whole gesture rather than a modifier — a ghost is
//! already short, an accent is already full length — so a set would mostly
//! express contradictions (an accented ghost) and would need an order of
//! application between its members to mean anything. The one real combination
//! it costs is *short and hard*, and that one is a written `dur` beside an
//! `accent`, which is what the field is for.

use serde::{Deserialize, Serialize};

/// How much harder than written an accent is struck.
///
/// A multiplier rather than a target, so an accent keeps the dynamic the
/// document wrote instead of flattening every accented note onto one level —
/// the same shape [`vel_scale`](super::Play::vel_scale) and
/// [`Humanize::velocity`](super::Humanize::velocity) already have, which is
/// what lets the three multiply in any combination without arguing.
///
/// A note already written at full velocity therefore comes out unaccented,
/// because full is full and the note renderer clamps there. That is the rule
/// `humanize` already lives by — a piece that wants dynamics writes its notes
/// below `1.0` — and it is deliberately **documented rather than refused**: a
/// note at `1.0` in a section carrying `"vel_scale": 0.6` has all the headroom
/// it needs, and a refusal read off the written value alone would refuse it.
const ACCENT_VELOCITY: f32 = 1.3;

/// How much brighter an accent is, as a fraction of the velocity it ends up
/// played at.
///
/// An accent is not merely a louder note: leaning on a string or hitting a key
/// squarely opens the instrument up as well. It reaches a patch through the two
/// routings that already read velocity as effort, so a patch naming neither
/// hears only the level — the same silence
/// [`Humanize::timbre`](super::Humanize::timbre) accepts, and for the same
/// reason: what "brighter" means belongs to the instrument.
const ACCENT_BRIGHTNESS: f32 = 0.15;

/// What fraction of its written `dur` a staccato note is actually held for.
///
/// Half, which is what the mark has meant on paper for two centuries. **The
/// written `dur` is untouched**, which is the whole point of spelling the
/// articulation instead of halving the number: the rhythm on the page stays the
/// rhythm of the music, and a phrase of quarter notes played short still reads
/// as quarter notes.
///
/// Fixed rather than a parameter. A fraction per note is the raw number this
/// module exists to replace, and a document that genuinely wants one specific
/// shorter length is writing a `dur`, which it can already do.
const STACCATO_GATE: f32 = 0.5;

/// How quiet a ghost is, as a fraction of the velocity written.
const GHOST_VELOCITY: f32 = 0.35;

/// What fraction of its written `dur` a ghost is held for.
///
/// Shorter than [`STACCATO_GATE`], and the two are separate numbers rather than
/// one shared constant because they are separate claims. A staccato note is a
/// note being played short; a ghost is barely a note at all.
const GHOST_GATE: f32 = 0.4;

/// How much duller a ghost is, as a fraction of the velocity it is played at.
///
/// Negative, and it is the field that makes a ghost *dead* rather than merely
/// quiet. Turning a note down alone leaves it the same bright thing further
/// away; a ghost is a muted string, so it loses its top as well.
const GHOST_BRIGHTNESS: f32 = -0.4;

/// Where a ghost sits against the beat, in seconds — negative, because it sits
/// ahead of it.
///
/// Seconds rather than beats, for the reason
/// [`Humanize::timing`](super::Humanize::timing) is in seconds: this is a hand
/// arriving early, not a subdivision, and it does not get three times wider
/// when the piece is played at 40 bpm. Twelve milliseconds is under a
/// thirty-second note at any tempo anybody writes and is plainly audible as
/// *ahead* rather than as a different rhythm.
const GHOST_EARLY: f32 = -0.012;

/// How a note is played: the mark a score would write over it.
///
/// One of a closed set of three; this module's own doc says what is
/// deliberately not in it, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Articulation {
    /// Struck harder than written, and brighter with it — the note that
    /// carries the groove.
    Accent,
    /// Held for half its written length, which stays written — the stop where
    /// a phrase breathes.
    Staccato,
    /// Quiet, short, dull and a hair early — the note between the notes.
    Ghost,
}

/// What an articulation does to one note, as four numbers.
///
/// One lookup per note rather than four, and one place where the whole meaning
/// of a mark is visible at once. [`PLAIN`](Self::PLAIN) is the absent
/// articulation, so the renderer has a single path rather than a branch per
/// note — the same shape an absent [`Humanize`](super::Humanize) already takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stroke {
    /// Multiplies the velocity the note is struck at.
    pub(crate) velocity: f32,
    /// Multiplies the gate, leaving the written `dur` alone.
    pub(crate) gate: f32,
    /// Moves the brightness routings away from the level, as a fraction of the
    /// velocity played — positive is brighter. Read through
    /// [`Stroke::timbre`].
    brightness: f32,
    /// Moves the onset, in seconds; negative is early.
    pub(crate) onset_seconds: f32,
}

impl Stroke {
    /// A note played exactly as written, which is what an absent articulation
    /// means.
    pub(crate) const PLAIN: Self = Self {
        velocity: 1.0,
        gate: 1.0,
        brightness: 0.0,
        onset_seconds: 0.0,
    };

    /// What an entry's articulation does, or [`PLAIN`](Self::PLAIN) if it
    /// carries none.
    pub(crate) fn of(articulation: Option<Articulation>) -> Self {
        articulation.map_or(Self::PLAIN, Articulation::stroke)
    }

    /// How far to move the velocity the *brightness* routings see, given the
    /// velocity `played` this note is actually struck at.
    ///
    /// A fraction of the played velocity rather than of full scale, which is
    /// the form [`Humanize::timbre`](super::Humanize::timbre) already uses and
    /// is why the two can simply be added: both are offsets in the same units,
    /// against the same number.
    pub(crate) fn timbre(self, played: f32) -> f32 {
        played * self.brightness
    }
}

impl Articulation {
    /// The four numbers this mark stands for.
    pub(crate) fn stroke(self) -> Stroke {
        match self {
            Self::Accent => Stroke {
                velocity: ACCENT_VELOCITY,
                brightness: ACCENT_BRIGHTNESS,
                ..Stroke::PLAIN
            },
            Self::Staccato => Stroke {
                gate: STACCATO_GATE,
                ..Stroke::PLAIN
            },
            Self::Ghost => Stroke {
                velocity: GHOST_VELOCITY,
                gate: GHOST_GATE,
                brightness: GHOST_BRIGHTNESS,
                onset_seconds: GHOST_EARLY,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An absent mark changes nothing at all — the note the renderer rendered
    /// before this module existed.
    #[test]
    fn no_articulation_is_the_identity() {
        let plain = Stroke::of(None);
        assert_eq!(plain, Stroke::PLAIN);
        assert_eq!(plain.velocity, 1.0);
        assert_eq!(plain.gate, 1.0);
        assert_eq!(plain.timbre(0.8), 0.0);
        assert_eq!(plain.onset_seconds, 0.0);
    }

    /// An accent is louder **and** brighter, and it is nothing else: it does
    /// not shorten the note and it does not move it.
    #[test]
    fn an_accent_is_harder_and_brighter_and_on_time() {
        let accent = Stroke::of(Some(Articulation::Accent));
        assert!(accent.velocity > 1.0, "an accent is struck harder");
        assert!(accent.timbre(0.6) > 0.0, "and opens the instrument up");
        assert_eq!(accent.gate, 1.0, "an accent is not a shorter note");
        assert_eq!(accent.onset_seconds, 0.0, "nor a displaced one");
    }

    /// Staccato is the gate and only the gate: the same note, held shorter.
    #[test]
    fn staccato_shortens_the_gate_and_nothing_else() {
        let short = Stroke::of(Some(Articulation::Staccato));
        assert!(short.gate < 1.0, "staccato is held short");
        assert_eq!(short.velocity, 1.0, "and at the velocity written");
        assert_eq!(short.timbre(0.6), 0.0, "with the tone written");
        assert_eq!(short.onset_seconds, 0.0, "where it was written");
    }

    /// The whole reason a ghost is not just a quiet note: every one of the four
    /// numbers moves, and each in the direction the name claims.
    #[test]
    fn a_ghost_is_quiet_and_short_and_dull_and_early() {
        let ghost = Stroke::of(Some(Articulation::Ghost));
        assert!(ghost.velocity < 1.0, "a ghost is quiet");
        assert!(ghost.gate < 1.0, "and short");
        assert!(ghost.timbre(0.6) < 0.0, "and dull");
        assert!(ghost.onset_seconds < 0.0, "and early");
    }

    /// A ghost is shorter than a staccato note, which is the claim
    /// [`GHOST_GATE`]'s own doc makes and the one a shared constant would
    /// silently drop.
    #[test]
    fn a_ghost_is_shorter_than_a_staccato_note() {
        let ghost = Stroke::of(Some(Articulation::Ghost));
        let short = Stroke::of(Some(Articulation::Staccato));
        assert!(ghost.gate < short.gate);
    }

    /// The brightness offset is a fraction of the velocity the note is played
    /// at, so a soft strike strays less in absolute terms than a hard one —
    /// the form that lets it be added to the humanise offset.
    #[test]
    fn the_brightness_offset_is_a_fraction_of_the_velocity_played() {
        for mark in [Articulation::Accent, Articulation::Ghost] {
            let stroke = Stroke::of(Some(mark));
            let loud = stroke.timbre(1.0);
            assert!((stroke.timbre(0.25) - loud * 0.25).abs() < 1e-6);
            assert!(loud.abs() > 1e-6, "{mark:?} moves the brightness at all");
        }
    }

    /// The names a document writes, which are the names the page documents.
    #[test]
    fn the_marks_are_spelled_as_the_page_spells_them() {
        for (mark, written) in [
            (Articulation::Accent, "\"accent\""),
            (Articulation::Staccato, "\"staccato\""),
            (Articulation::Ghost, "\"ghost\""),
        ] {
            let json = serde_json::to_string(&mark).expect("a mark serialises");
            assert_eq!(json, written);
            let read: Articulation = serde_json::from_str(written).expect("reads back");
            assert_eq!(read, mark);
        }
        assert!(serde_json::from_str::<Articulation>("\"legato\"").is_err());
    }
}
