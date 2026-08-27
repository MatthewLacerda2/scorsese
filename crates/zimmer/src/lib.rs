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
//! Everything renders at [`SAMPLE_RATE`], in stereo. Both are fixed rather
//! than chosen per call, and they are fixed for **different reasons** — which
//! is worth separating, because one argument does not carry the other.
//!
//! The **rate** is fixed because a bake is addressed by the hash of its recipe,
//! so it must not depend on what some later render happens to ask for. A rate
//! that followed the render would give one recipe many different files under
//! one hash.
//!
//! That argument says nothing about **channels**: it asks only that the count
//! be fixed, and two is as fixed as one. Two is chosen on its own terms, and
//! the terms are fidelity. A mono, centred, dry mix is not merely narrower
//! than a stereo one — it is a specific and dated sound, because everything a
//! listener hears as *produced* is width: a reverb arriving from both sides, a
//! pad spread against a centred bass, hats sitting slightly off-axis. Without
//! any of it, five instruments are five things stacked at one point rather
//! than a mix. This crate was mono until [`SYNTH_VERSION`] 3, on a simplicity
//! argument that was true and did not weigh that bill; the width is worth what
//! it costs, and what it costs is the paragraph below.
//!
//! **The samples are two `Vec<f32>`, not one `Vec<[f32; 2]>`.** The
//! alternative — interleaved frames — was the other real candidate, and it
//! loses on what this crate is mostly made of: functions that take
//! `&mut [f32]` and walk it. A filter, an envelope, a waveshaper, a delay
//! line, the polyBLEP oscillator stack: every one of them is a mono algorithm
//! with mono state, and under the split form every one of them stays written
//! exactly that way, with `Stereo::each` handing it one channel at a time. The
//! interleaved form would have made each of them either stride by two and
//! carry an array of its own state, or be wrapped in a de-interleave; the
//! first is a change to every module and the second is the split form, paid
//! for twice. The stages that genuinely need both sides at once are countable
//! — the linked limiter, the reverb's mono send, the pan — and they reach for
//! `l` and `r` by name, which reads as what it is. Interleaving then happens
//! at exactly two edges, the WAV encoder and the meter, where a whole buffer
//! was wanted anyway.
//!
//! **Width belongs to the mix, never to a note.** A source is one signal: an
//! oscillator stack, a plucked string and an FM pair each produce a single
//! waveform, and it reaches both channels identically. `noise` is the one
//! exception — an uncorrelated second draw is a free and legitimate way to
//! make a noise source wide, and the seeded hash already takes a channel
//! discriminator. Everything else that widens a sound happens downstream, in
//! the places a mix decision lives: a track's `pan`, and the stereo reverb.
//! There is no per-note pan and no mid/side processing, for the same reason
//! there is no node graph — that is a compositing suite's craft, not an
//! editor's.
//!
//! `scorsese-render` resamples and upmixes every audio source on the way into
//! the mix — a synthesised file takes exactly the path an imported stereo file
//! does.

pub(crate) mod core;
pub(crate) mod error;
pub(crate) mod fx;
pub(crate) mod hash;
pub mod level;
pub(crate) mod note;
pub mod patch;
pub mod song;
pub(crate) mod stereo;
pub mod survey;
pub mod wav;

pub use core::SAMPLE_RATE;
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
///
/// **Version 3 is the far end of that scale** and needs no argument at all: the
/// crate went stereo, so every bake in every project is a different file, of a
/// different channel count, and there is no recipe anywhere that renders to
/// what it used to.
pub const SYNTH_VERSION: u32 = 3;

/// Render one note of `patch` and encode it as a stereo 16-bit PCM WAV.
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

/// Render one note of `patch`, handing back the samples rather than a file.
///
/// **Interleaved stereo**, left sample first — the form a WAV holds and the
/// form [`level`] measures, which is what every caller of raw samples in this
/// workspace already speaks. The split pair the signal path itself works in is
/// this crate's own business; see [the crate doc](self).
///
/// Pre-limiter, unlike [`bake_note`]: a caller summing several of these wants
/// to limit the sum rather than each part of it.
pub fn render_note(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Vec<f32>, SynthError> {
    Ok(core::render_note(patch, midi, opts)?.interleaved())
}

/// Render `song` and encode it as a stereo 16-bit PCM WAV.
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
    /// The encoded file, stereo 16-bit PCM.
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

    /// Encodes and measures one signal, cutting it where `sections` says and
    /// carrying the rows its layers were already measured into.
    ///
    /// The channels are woven together once and the file and the measurement
    /// are both taken from that one buffer, because they must agree: a report
    /// that measured the samples and a file that carried different ones would
    /// be two answers to one question.
    fn of(samples: &stereo::Stereo, sections: Vec<level::Cut>, tracks: Vec<level::Layer>) -> Self {
        let interleaved = samples.interleaved();
        let mut profiler = level::Profiler::sectioned(stereo::CHANNELS, SAMPLE_RATE, sections);
        profiler.feed(&interleaved);
        Self {
            wav: wav::encode(&interleaved),
            profile: profiler.finish(),
            tracks,
        }
    }
}
