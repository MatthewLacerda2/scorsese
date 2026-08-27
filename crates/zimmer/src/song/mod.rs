//! The song document: patterns, arrangement, tracks.
//!
//! Where [`crate::patch`] describes *one instrument*, a [`Song`] describes *a
//! piece of music*: which instruments play (tracks), what they play (patterns
//! of notes), and in what order (the arrangement).
//!
//! The shape is **tracker-style** (the MOD/XM lineage), not a linear piano
//! roll, and deliberately so: a melody with any repetition in it is a handful
//! of short patterns and a list naming them, which is a few dozen lines an
//! agent can write, diff and iterate on. The same music as a flat note list
//! would be hundreds of lines with the repetition spelled out, and every edit
//! would be a merge conflict with itself.
//!
//! Everything is **beats**, never seconds — `bpm` converts once at render time
//! — so changing the tempo of a finished song is one number.
//!
//! ```jsonc
//! {
//!   "bpm": 120,
//!   "seed": 7,
//!   "key": "E minor",
//!   "tracks": [{ "name": "bass", "patch": "recipes/bass.json", "gain": 0.8 }],
//!   "patterns": {
//!     "verse": { "beats": 8, "notes": [
//!       { "track": "bass", "note": "E2", "start": 0.0, "dur": 0.5, "vel": 1.0 },
//!       { "track": "lead", "degree": 5, "oct": 4, "start": 1.0, "dur": 0.5 },
//!       { "track": "keys", "chord": "Em7", "oct": 3, "start": 0.0, "dur": 2.0 },
//!       { "track": "hat", "steps": "x-x-x-x-x-x-x-xX", "div": 0.5, "vel": 0.4 }] }
//!   },
//!   "arrangement": ["verse", { "pattern": "verse", "transpose_degrees": 1 }]
//! }
//! ```
//!
//! An arrangement entry is a pattern's name, or that name with transforms: a
//! repeat that can vary is the difference between music that develops and music
//! that only repeats.
//!
//! An entry in a pattern is one note — written absolutely, or as a degree of
//! the song's [`Key`] — or one [`Chord`], or one run of [`Steps`]. That is the
//! same idea one level down, and for the same reason: four notes that are one
//! chord, or sixteen that are one hi-hat part, should be one thing in the
//! document, or nothing records that they are; and a melody that is in a key
//! should say so once rather than two hundred times.
//!
//! Rendering lives in [`render_song`]; this file is the document alone — plain
//! serde data that round-trips losslessly, the same "document as truth" rule
//! the patch follows.

pub(crate) mod arrangement;
pub(crate) mod chord;
pub(crate) mod feel;
pub(crate) mod key;
mod mix;
pub(crate) mod render;
pub(crate) mod sections;
pub(crate) mod shape;
pub(crate) mod steps;
pub(crate) mod timing;
mod validate;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::SynthError;
use crate::patch::{Fx, Patch};

pub use arrangement::{ArrangementEntry, Play};
pub use chord::{Chord, Voicing};
pub use feel::Humanize;
pub use key::{Degree, DegreeNote, Key, Mode};
pub use render::{InlineOnly, PatchResolver, render_song};
pub use steps::Steps;
pub use timing::{Fade, Fit, FitMode, Tail};

/// Default for a per-track or per-note gain: unity, i.e. "as written".
fn one() -> f32 {
    1.0
}

/// Whether a song swings at all — the test that keeps `"swing": 0.0` out of
/// every saved document.
///
/// A field that is absent when it does nothing matters more here than
/// elsewhere: a bake is addressed by the hash of the recipe's bytes, alongside
/// [`SYNTH_VERSION`](crate::SYNTH_VERSION), so a serialiser that started
/// writing a default into every song would invalidate every cached bake in
/// every project at once, for no change in the audio — which is the cost this
/// crate pays deliberately when the audio *does* change, and never otherwise.
fn no_swing(swing: &f32) -> bool {
    *swing == 0.0
}

/// Whether a track sits dead centre — the test that keeps `"pan": 0.0` out of
/// every saved document, for the reason [`no_swing`] gives.
fn centred(pan: &f32) -> bool {
    *pan == 0.0
}

/// A complete piece of music, renderable to one stereo buffer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// Tempo in beats per minute; the one place beats become seconds.
    pub bpm: f32,
    /// Folded into every note's render seed, so one number re-rolls every
    /// stochastic source in the piece while keeping it reproducible.
    #[serde(default)]
    pub seed: u64,
    /// What key the piece is in — `"D minor"`, `"F# lydian"` — which is what
    /// lets a note be written as a **degree** and an arrangement entry lift a
    /// section **within** the key rather than out of it. Absent means the song
    /// declares no key, and everything behaves exactly as it did before the
    /// field existed: absolute pitches, chromatic transposes.
    ///
    /// Kept as the text it was written as, resolved through [`Song::key`], the
    /// way a [`Pitch::Name`] is kept and resolved — so a document round-trips
    /// as its author spelled it and `Db minor` is never saved back as
    /// `C# minor`. [`Key`] has the numbering, the modes and the reasoning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The instruments, in a fixed order — the order is part of the seed
    /// derivation, so it is data, not presentation.
    pub tracks: Vec<Track>,
    /// Named blocks of notes. A `BTreeMap` rather than a `HashMap` so
    /// [`Song::to_json`] emits patterns in a stable order and two saves of the
    /// same song are byte-identical.
    pub patterns: BTreeMap<String, Pattern>,
    /// Which patterns play, in order. An entry is a pattern's name, or that
    /// name with [transforms](ArrangementEntry) — a repeat that varies rather
    /// than photocopies.
    pub arrangement: Vec<ArrangementEntry>,
    /// How far the off-beat eighths sit behind the grid: `0.0` is straight,
    /// `0.33` is roughly the triplet feel, `0.5` is dotted. A property of the
    /// *performance*, so it is song-level — a rhythm section that swings while
    /// the lead does not is a specific effect, not a default. Onsets only: a
    /// note's `dur` is what the document says it is.
    #[serde(default, skip_serializing_if = "no_swing")]
    pub swing: f32,
    /// How far the player strays from the written page in timing and
    /// velocity. Absent means a machine plays the part, which is what every
    /// song did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humanize: Option<Humanize>,
    /// Effects on the finished mix, applied to the sum of every track — and
    /// *before* the master limiter, which is the only place they can run
    /// without withdrawing the promise that a bake cannot clip. This is where a
    /// piece gets a **room**: one reverb over everything, decaying once, rather
    /// than the same settings copied into every patch and drifting apart the
    /// first time one of them is tuned. It is also where a piece gets **glue**:
    /// a modest [`Fx::Saturate`] over the sum pushes every track through one
    /// curve, which is much of what stops a mix sounding like separate parts
    /// added together. Empty means the mix reaches the limiter exactly as it
    /// was summed. The `song::mix` module has the reasoning.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fx: Vec<Fx>,
    /// A length the piece has to come out at, when the picture decides that
    /// rather than the music. Absent means the song is as long as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<Fit>,
    /// Level moves on the finished piece. Absent means neither end moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade: Option<Fade>,
    /// What happens after the last beat. Absent means [`Tail::Ring`], which is
    /// what every song did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<Tail>,
}

/// One instrument in the mix: a patch, and how loud it sits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// How notes refer to this track.
    pub name: String,
    /// The instrument it plays: inline, or named for a resolver to find.
    pub patch: PatchRef,
    /// Linear gain applied to every note this track plays. The master limiter
    /// guarantees the sum never clips, so this is a balance control, not a
    /// safety one.
    #[serde(default = "one")]
    pub gain: f32,
    /// Where this instrument sits across the stereo image: `-1.0` hard left,
    /// `0.0` centre, `1.0` hard right. Out-of-range values are clamped rather
    /// than refused — there is no position past hard over for them to mean.
    ///
    /// A property of the *mix*, so it is per track and not per note. Five
    /// instruments all at zero is five things stacked at one point in space,
    /// which is most of what makes a score sound unproduced; spreading a pad
    /// against a centred bass and kick is the cheapest width there is.
    ///
    /// **Constant power** (see [`Song`]'s renderer), so moving a part off
    /// centre does not quieten it, and centre is exactly unity — a track that
    /// never mentions this field renders the samples it always did.
    #[serde(default, skip_serializing_if = "centred")]
    pub pan: f32,
    /// Effects on this instrument's whole part, applied to the sum of its
    /// notes *before* [`Track::gain`] reaches the master — a delay that
    /// belongs to the keys rather than to one chord of them. Distinct from
    /// [`Patch::fx`], which is the instrument's own sound and is applied to
    /// each note separately — the `song::mix` module has the ordering between
    /// the two, which is the load-bearing part. Empty means this track's notes
    /// are added straight into the master, exactly as they were before the
    /// field existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fx: Vec<Fx>,
}

/// A track's instrument: the patch inline, or a reference for the caller to
/// resolve.
///
/// A song being iterated on carries its patches inline and stays one
/// self-contained file; a song built from a settled instrument library names
/// them and stays short.
///
/// What a reference *means* is not this crate's business — it resolves through
/// a [`PatchResolver`] the caller supplies. In scorsese that is a path under
/// the project root, checked by the same rules every other path obeys; nothing
/// here opens a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatchRef {
    /// A name for the caller's resolver to turn into a patch.
    Named(String),
    /// The patch document itself. Boxed because it dwarfs the other variant.
    Inline(Box<Patch>),
}

/// A named block of notes, `beats` long.
///
/// Patterns are just N beats — no time signature, because nothing in the
/// renderer needs bars.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    /// How long this block occupies in the arrangement, in beats. Notes may
    /// ring out past it — the next pattern starts on time regardless — so this
    /// is the *slot*, not the sound.
    pub beats: f32,
    /// What is played, and when within the slot.
    pub notes: Vec<PatternEntry>,
}

impl Pattern {
    /// Every note this pattern plays — chords expanded to their voices, step
    /// strings to their hits, degrees resolved against the song's key — in
    /// document order.
    ///
    /// The renderer works from this rather than from the written entries, so
    /// every one of those is an ordinary note by the time swing, humanise and
    /// the per-note seed reach it — see [`chord`], [`steps`] and [`key`].
    pub(crate) fn voices(&self, key: Option<&Key>) -> Result<Vec<Note>, SynthError> {
        let around = Context {
            beats: self.beats,
            key,
        };
        let mut voiced = Vec::with_capacity(self.notes.len());
        for entry in &self.notes {
            entry.voice_into(around, &mut voiced)?;
        }
        Ok(voiced)
    }
}

/// What an entry is expanded *against*: the slot it sits in, and the key the
/// song is in.
///
/// One parameter rather than one per fact, because each fact is read by
/// exactly one kind of entry — a step string checks its grid against `beats`,
/// a degree looks itself up in `key`, and a plain note reads neither. Passed
/// separately those would be a list of arguments that grows by one every time
/// a notation is added and is ignored by every variant but the new one, which
/// is how a signature stops describing anything. Named, it says the one thing
/// that is true of all of them: an entry means what it means in a context, and
/// this is the whole of that context.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Context<'a> {
    /// The slot of the pattern holding the entry, in beats.
    pub(crate) beats: f32,
    /// The key the song declared, if it declared one.
    pub(crate) key: Option<&'a Key>,
}

/// One entry in a pattern, in the four forms one is written in.
///
/// They are **two pairs**, which is the order they are declared in: two ways to
/// write a single note — absolutely, or as a [`Degree`] of the song's key — and
/// two ways to write several notes as one idea, [`Chord`] for the harmony and
/// [`Steps`] for the rhythm. Every one of them expands to ordinary notes before
/// a sample is produced, so this enum is about how a part is *written*, and
/// nothing past that expansion knows which form the page used.
///
/// Untagged, told apart by **which field is present** — `note`, `degree`,
/// `chord` or `steps` — the way [`Pitch`] tells a name from a MIDI number.
/// Every variant [denies unknown fields](https://serde.rs/container-attrs.html),
/// which is what keeps that discrimination honest as the list grows: an entry
/// carrying two of the four is refused by all four rather than resolved by
/// declaration order, and so is one with a misspelled field. Guessing between
/// them is the one outcome worse than any of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatternEntry {
    /// One note, exactly as patterns have always been written.
    Note(Note),
    /// One note, named as a degree of the song's [key](Song::key) and resolved
    /// to a pitch before anything is rendered.
    Degree(DegreeNote),
    /// A chord, expanded to its voices before anything is rendered.
    Chord(Chord),
    /// A run of evenly spaced hits, expanded to notes before anything is
    /// rendered.
    Steps(Steps),
}

impl PatternEntry {
    /// The [`Track::name`] that plays it.
    pub fn track(&self) -> &str {
        match self {
            Self::Note(note) => &note.track,
            Self::Degree(degree) => &degree.track,
            Self::Chord(chord) => &chord.track,
            Self::Steps(steps) => &steps.track,
        }
    }

    /// Onset in beats from the start of its pattern.
    pub fn start(&self) -> f32 {
        match self {
            Self::Note(note) => note.start,
            Self::Degree(degree) => degree.start,
            Self::Chord(chord) => chord.start,
            Self::Steps(steps) => steps.start,
        }
    }

    /// Gate length in beats — of every hit, for an entry that is several.
    pub fn dur(&self) -> f32 {
        match self {
            Self::Note(note) => note.dur,
            Self::Degree(degree) => degree.dur,
            Self::Chord(chord) => chord.dur,
            Self::Steps(steps) => steps.gate(),
        }
    }

    /// Appends what this entry sounds as to `out` — one note, the pitch a
    /// degree names, a chord's voices low to high, or a step string's hits in
    /// time order. Nothing is appended when it is refused.
    ///
    /// Each form reads the part of [`Context`] it needs and no more: a step
    /// string checks its grid against the pattern's slot, which is the one
    /// thing standing between a string one character short and a bar that
    /// quietly is not one; a degree looks itself up in the key, and is refused
    /// when the song declared none.
    pub(crate) fn voice_into(
        &self,
        around: Context<'_>,
        out: &mut Vec<Note>,
    ) -> Result<(), SynthError> {
        match self {
            Self::Note(note) => {
                out.push(note.clone());
                Ok(())
            }
            Self::Degree(degree) => degree.voice_into(around.key, out),
            Self::Chord(chord) => chord.voice_into(out),
            Self::Steps(steps) => steps.voice_into(around.beats, out),
        }
    }
}

impl From<Note> for PatternEntry {
    fn from(note: Note) -> Self {
        Self::Note(note)
    }
}

impl From<DegreeNote> for PatternEntry {
    fn from(degree: DegreeNote) -> Self {
        Self::Degree(degree)
    }
}

impl From<Chord> for PatternEntry {
    fn from(chord: Chord) -> Self {
        Self::Chord(chord)
    }
}

impl From<Steps> for PatternEntry {
    fn from(steps: Steps) -> Self {
        Self::Steps(steps)
    }
}

/// One note: which track plays it, what pitch, when, for how long, how hard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    /// The [`Track::name`] that plays it.
    pub track: String,
    /// What pitch to play.
    pub note: Pitch,
    /// Onset in beats from the start of its pattern.
    pub start: f32,
    /// Gate length in beats. The patch's release rings out after it.
    pub dur: f32,
    /// Velocity in `0..=1`, scaling this note's peak amplitude.
    #[serde(default = "one")]
    pub vel: f32,
}

/// A pitch, written the way a score does or as a raw MIDI number.
///
/// Untagged, and `Name` first: a JSON string can only be a name and a JSON
/// number can only be a MIDI value, so the two never race.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Pitch {
    /// Scientific pitch notation — `"C#4"`, `"Bb3"` (see
    /// [`parse_note`](crate::parse_note)).
    Name(String),
    /// A MIDI note number; fractional values are legal microtonal pitches.
    Midi(f32),
}

impl Pitch {
    /// Resolves to a MIDI number, reporting an unparseable name.
    pub fn to_midi(&self) -> Result<f32, SynthError> {
        match self {
            Pitch::Name(name) => crate::note::parse_note(name),
            Pitch::Midi(midi) => Ok(*midi),
        }
    }
}

impl Song {
    /// Serialises to canonical JSON — pretty, with patterns in name order.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parses a song from its JSON form.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Total arrangement length in beats — the sum of its patterns' slots.
    ///
    /// Notes are allowed to ring out past this: it is where the *last pattern*
    /// ends, not where the audio does.
    pub fn arrangement_beats(&self) -> f32 {
        self.arrangement
            .iter()
            .filter_map(|entry| self.patterns.get(entry.pattern()))
            .map(|pattern| pattern.beats)
            .sum()
    }

    /// Seconds per beat.
    pub fn beat_seconds(&self) -> f32 {
        60.0 / self.bpm
    }

    /// What this song does after its last beat, defaults included — so a
    /// caller never has to decide what an absent field meant.
    pub fn tail(&self) -> Tail {
        self.tail.unwrap_or_default()
    }

    /// The key this song declares, read, or a refusal if it is not a key.
    ///
    /// `Ok(None)` is a song that declares none, which is a legal and complete
    /// document — it just cannot write a degree or lift a section diatonically.
    pub fn key(&self) -> Result<Option<Key>, SynthError> {
        self.key.as_deref().map(Key::parse).transpose()
    }

    /// How far this song's player strays from the page, defaults included.
    /// An absent field is a [`Humanize`] that scatters nothing, so the
    /// renderer has one path rather than two.
    pub fn humanize(&self) -> Humanize {
        self.humanize.unwrap_or_default()
    }
}
