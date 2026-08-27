//! # scorsese-zimmer — sound from a document
//!
//! Responsibility: turning a **recipe** — a synthesiser patch, or a song —
//! into samples. The oscillators, filters, envelopes and effects that make one
//! note; the tracker-style arrangement that makes a piece of music out of many;
//! and the WAV encoding that puts either into bytes.
//!
//! **Effects and score, and no voice.** What this crate makes is sound nobody
//! speaks: a gunshot, a footstep, a UI blip, and the music under all of them.
//! Speech is not a feature that is merely unimplemented here — it is a
//! different craft, and neither [`Patch`] nor [`Song`] has anywhere to put a
//! phoneme. A project that needs narration gets it from a provider, which is a
//! fact about the project rather than about this crate.
//!
//! This is the free half of scorsese's generated content. A `synth_audio`
//! asset costs no money, needs no key and no network, and produces the **same
//! bytes every time** — *given a fixed synthesiser*, which is a real
//! qualifier and not a hedge. A recipe is a pure function of its document, and
//! this crate is the other argument to it: change a filter here and every
//! recipe in every project renders to something new. That is what
//! [`SYNTH_VERSION`] is for, and why the address of a bake carries it
//! alongside the hash of the recipe.
//!
//! ## Boundary
//!
//! This crate performs **no I/O of any kind**: no filesystem, no network, no
//! process, no display. It takes documents and returns buffers and bytes; a
//! caller writes them. Nor does it depend on `scorsese-core` — it has never
//! heard of a project, an asset or a `ProjectPath`, and a song that names its
//! instruments by reference resolves them through a caller-supplied
//! [`PatchResolver`] rather than by opening anything itself.
//!
//! That boundary is what makes the determinism claim checkable: every output
//! is a pure function of the documents handed in.
//!
//! ## What this publishes
//!
//! **The crate root is the way in.** [`bake_note`], [`bake_named_note`] and
//! [`bake_song`] each take a document and hand back a [`Bake`] — a finished
//! file and how it came out. [`render_note`] and [`render_song`] are the same
//! two jobs stopping one step earlier, at the samples, for a caller that wants
//! to do something with them other than write them down. Around those sit the
//! documents they take ([`Patch`], [`Song`], [`NoteOpts`]), the
//! [`PatchResolver`] a song's references resolve through, [`SynthError`],
//! [`SAMPLE_RATE`], [`SYNTH_VERSION`], and [`parse_note`] and [`midi_to_freq`]
//! for turning what a score writes into what the renderer plays.
//!
//! **Five modules keep their own path**, because each is a vocabulary rather
//! than a handful of names:
//!
//! - [`patch`] and [`song`] are the two recipe documents — every type a
//!   `recipes/*.json` file deserialises into. They are named for the document
//!   they belong to, and `song::Note` and `patch::Osc` say what they are in a
//!   way that thirty more names at the root would not.
//! - [`level`] is the measurement, and it is here rather than beside a caller
//!   because a render measuring its finished mix and a bake measuring what it
//!   just synthesised are the same arithmetic. `scorsese-render` imports it
//!   whole.
//! - [`survey`] is what a *set* of recipes is made of, counted from the
//!   documents without baking any of them.
//! - [`wav`] publishes one function, [`wav::seconds_in`], and keeps its module
//!   because the noun is what makes the verb readable.
//!
//! **Everything else is `pub(crate)`.** The oscillators, filters, envelopes,
//! effects, the seeded hash and the note-name parser are the inside of
//! [`render_note`]; the song mixer's per-track intermediate is the inside of
//! [`bake_song`]. None of it is a second way to reach a result this crate
//! already returns.
//!
//! ## The signal path
//!
//! A [`Patch`] is *structured, not a free graph*. The stages are fixed and the
//! recipe chooses what fills them — never how they connect:
//!
//! ```text
//!   source ─► filter ─► amp envelope ─► fx chain
//!      ▲         ▲            ▲
//!      └─────── LFO ──────────┘   (one target: pitch | cutoff | amp)
//! ```
//!
//! A [`Song`] layers that: tracks name patches, patterns name notes, and the
//! arrangement names patterns. Every time in a song is in **beats**, so
//! changing the tempo of a finished piece is one number.
//!
//! ## Determinism
//!
//! Nothing here reads a clock or a random number generator. Every stochastic
//! element — noise, the Karplus excitation — draws from one seeded integer
//! hash, so the same recipe and seed produce identical samples in any process,
//! on any machine, on any run — *of this version of this crate*. Determinism
//! across versions is not claimed and never was: a change to a source, a
//! filter or an effect changes what every recipe renders to, deliberately.
//! [`SYNTH_VERSION`] is how that shows up outside, and bumping it is part of
//! making such a change.
//!
//! ## Rate and channels
//!
//! Everything renders at [`SAMPLE_RATE`], in mono. Both are deliberate rather
//! than limitations to work around, and they are deliberate for **different
//! reasons** — which is worth separating, because one argument does not carry
//! the other.
//!
//! The **rate** is fixed because a bake is addressed by the hash of its recipe,
//! so it must not depend on what some later render happens to ask for. A rate
//! that followed the render would give one recipe many different files under
//! one hash.
//!
//! That argument says nothing about **channels**: it asks only that the count
//! be fixed, and two is as fixed as one. Mono is chosen on its own terms, and
//! the terms are simplicity. Every buffer in this crate is one `Vec<f32>`, so
//! every source, filter, envelope and effect is a single-buffer function; the
//! limiter clamps one signal rather than linking two so a stereo image does not
//! lurch when it acts; and no recipe, and no agent writing one, ever has to
//! think about width. **Stereo and panning are declined, not deferred** — a
//! song-level effects bus is where they would naturally arrive, and they do not
//! arrive there either.
//!
//! `scorsese-render` resamples and upmixes every audio source on the way into
//! the mix — a synthesised file takes exactly the path an imported mono file
//! does.

pub(crate) mod core;
pub(crate) mod error;
pub(crate) mod fx;
pub(crate) mod hash;
pub mod level;
pub(crate) mod note;
pub mod patch;
pub mod song;
pub mod survey;
pub mod wav;

pub use core::{SAMPLE_RATE, render_note};
pub use error::SynthError;
pub use note::{NoteOpts, midi_to_freq, parse_note};
pub use patch::Patch;
pub use song::{PatchResolver, Song, render_song};

/// The synthesiser's own version: the number that changes when the same
/// recipe would render to different samples.
///
/// A bake is addressed by a digest of its recipe **and** this number, so
/// bumping it is what makes every affected file miss the cache and be
/// re-rendered. Without it the address describes only the document, and a
/// project keeps serving audio its own recipe no longer describes.
///
/// It is **declared, not derived**. The tempting derivation — hash the
/// rendered output of a few probe recipes — cannot work here: every voice in
/// this crate rides on the platform's `sin`, `exp` and `powf`, which are not
/// bit-identical between a glibc box and a Mac, and a digest has no tolerance
/// to spend. A derived version would announce a synthesiser change every time
/// a project moved machine, which is the opposite of what an address is for.
///
/// So it is a number a person has to bump, and the rule is the one
/// `schema_version` already lives by: **bump it in the same change that makes
/// a recipe render differently**, and let everything break loudly.
///
/// **What "differently" has meant so far**, because the first change to face
/// this question was an ambiguous one and the answer is easier to reuse than
/// to re-derive: *any* representable recipe, not *most* recipes and not an
/// audible difference. Version 2 added curved envelope segments and a pitch
/// envelope, both no-ops at their defaults — 23 of 25 probe recipes came back
/// byte-identical, every envelope corner and every sane vibrato among them.
/// The two that moved were pitch LFOs deeper than the ten octaves the
/// frequency track now bounds, which render nothing but aliasing either way.
/// It was bumped regardless. A spurious bump costs one re-render of audio that
/// turns out identical; a missed one leaves a project serving samples its
/// recipe no longer describes, which is the failure this constant exists to
/// prevent — so where the two are in tension, bump.
///
/// **What has not moved it**, since the easy half of the rule is worth writing
/// down beside the hard half: a **new** effect, source or stage that an
/// existing recipe cannot be using. The EQ arrived as a new `Fx` variant and a
/// module nothing calls without one, and no field of any existing variant
/// changed — 25 probe recipes covering every source, every envelope corner,
/// every LFO target and every existing effect came back byte-identical across
/// the change. A recipe that does not name the new thing renders what it
/// always did, by construction rather than by luck, and that is the case where
/// leaving this number alone is right.
///
/// **New notation is the same case**, and worth naming separately because it
/// does not look like it. A song key and scale degrees added a `Song` field
/// and a fourth kind of pattern entry, reordered the enum holding the other
/// three, and replaced the note loop's pitch step — and 18 probes across the
/// arrangement transforms, swing, all three humanise axes, the fit modes, the
/// fades, both kinds of chain, all four sources, chords and step strings still
/// came back byte-identical. The measure is the same one either way: *can a
/// recipe written before this change name the new thing?* Not *did the code
/// around it move*.
pub const SYNTH_VERSION: u32 = 2;

/// Render one note of `patch` and encode it as a mono 16-bit PCM WAV.
///
/// The one-call front door for an effect: a gunshot, a footstep, a UI blip.
/// The returned bytes are a complete file — a caller writes them wherever the
/// project keeps its bakes.
///
/// The output is limited before encoding, always. A bake must not clip, and
/// that is not the recipe's decision to make.
pub fn bake_note(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Bake, SynthError> {
    // Neither sections nor tracks: a one-shot is one gesture played by one
    // voice, so both tables would be the summary printed a second time.
    Ok(Bake::of(
        &core::render_limited(patch, midi, opts)?,
        Vec::new(),
        Vec::new(),
    ))
}

/// Render one note of `patch`, naming the note the way a score does (`"C#4"`,
/// or a MIDI number as text), and encode it as a WAV.
pub fn bake_named_note(patch: &Patch, note: &str, opts: &NoteOpts) -> Result<Bake, SynthError> {
    bake_note(patch, parse_note(note)?, opts)
}

/// Render `song` and encode it as a mono 16-bit PCM WAV.
///
/// `resolve` supplies the patch behind any track that names its instrument by
/// reference rather than carrying it inline — see [`PatchResolver`].
pub fn bake_song(song: &Song, resolve: &dyn PatchResolver) -> Result<Bake, SynthError> {
    // The song's own arrangement decides the rows of the report, which is what
    // makes a row say "the second chorus is the quiet one" rather than "seconds
    // 24 to 32 are quiet". Its tracks decide the other table: which instrument
    // is taking up the room, which is the half a section row cannot answer.
    let mixed = song::render::mix_song(song, resolve)?;
    Ok(Bake::of(
        &mixed.master,
        song::sections::of(song),
        mixed.tracks,
    ))
}

/// A finished bake: the file, and how it came out.
///
/// The measurement rides along rather than being a second call, because the
/// only place the samples exist is inside the bake — handing back bytes alone
/// would mean decoding the WAV again to learn something that was in hand a
/// moment earlier. A recipe's level is a property of the recipe, so catching it
/// here is catching it before it is ever mixed.
#[derive(Debug, Clone, PartialEq)]
pub struct Bake {
    /// The encoded file, mono 16-bit PCM.
    pub wav: Vec<u8>,
    /// How it came out — over its whole length, and section by section.
    pub profile: level::Profile,
    /// How each track came out on its own, post-gain — which layer is taking
    /// up the room. Empty unless this was a song of more than one track: one
    /// row under a one-line summary is the same sentence twice.
    pub tracks: Vec<level::Layer>,
}

impl Bake {
    /// How loud the whole thing was.
    pub fn loudness(&self) -> &level::Loudness {
        self.profile.loudness()
    }

    /// Encodes and measures one buffer, cutting it where `sections` says and
    /// carrying the rows its layers were already measured into.
    /// Mono, as everything this crate makes is.
    fn of(samples: &[f32], sections: Vec<level::Cut>, tracks: Vec<level::Layer>) -> Self {
        let mut profiler = level::Profiler::sectioned(1, SAMPLE_RATE, sections);
        profiler.feed(samples);
        Self {
            wav: wav::encode(samples),
            profile: profiler.finish(),
            tracks,
        }
    }
}
