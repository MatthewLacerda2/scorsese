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
//! two jobs stopping one step earlier, at the samples — interleaved stereo, as
//! a file holds them — for a caller that wants to do something with them other
//! than write them down. Around those sit the
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
//! argument that was true and did not weigh that bill. What the width costs is
//! a second pass of arithmetic over a channel that, for every source but one,
//! is identical to the first — which is a price an offline renderer can pay,
//! and `core`'s own doc is where it is charged.
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
//! oscillator stack, a plucked string and an FM voice — of two operators or of
//! four — each produce a single waveform, and it reaches both channels
//! identically. `noise` is the one
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
/// The **compressor** is the second one of those, and it went further into the
/// mixer than the EQ did — a new buffer per keyed track, and a second entry
/// point into the fx chain — so it was checked rather than argued: 24 probes
/// across all four sources, every envelope corner, all three LFO targets,
/// every existing effect, all three chain locations, chords, step strings,
/// keys and degrees, swing and humanise, the arrangement transforms and the
/// fit, fade and tail fields came back byte-identical. The extra buffer is
/// only kept for a track something names, and nothing names one in a recipe
/// written before this existed.
///
/// The **chorus** is the third, and the plainest of the three: a new `Fx`
/// variant, two modules nothing calls without one, and no field of any
/// existing variant touched. Checked anyway, against `84ad2a3` — 30 probes
/// covering all four sources, every envelope corner including curves and a
/// pitch envelope, all three LFO targets, a velocity-routed filter, every
/// effect that existed then wet and dry, a chain of four, and eight songs
/// spanning swing, humanise, a track bus, a song chain, a fade, a `fit`, an
/// arrangement transform, a key with degrees and a step string. All 30 came
/// back byte-identical.
///
/// **A new *source* is the same case as a new effect**, and it was checked the
/// same way rather than reasoned from that sentence. `additive` added a
/// [`Source`](patch::Source) variant, a `Partial` type, a renderer module and
/// four refusals reachable only from the new arm; 46 probes — every source,
/// all three envelope curves, all three LFO targets, both filter kinds, a
/// pitch envelope, every effect and a two-stage chain, each at two velocities,
/// plus six songs covering swing, all three humanise axes, a pan, a track bus,
/// a song chain, a fade, an arrangement transform, a `fit` and a key, as
/// `360ac75` had them — came back byte-identical. The measure is the one above and not
/// the size of the diff: *can a recipe written before this change name the new
/// thing?*
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
/// **`fm4` is the second new source, and it needed a wider probe than the
/// first**, which is the part worth writing down rather than the verdict. The
/// verdict is the one above: a recipe written before it cannot name `fm4`, so
/// the number does not move.
///
/// What made the check different is that this one **touched code an existing
/// recipe does reach**. The source stage takes a new argument — the gate, for
/// the per-operator envelopes — so every source's call site moved; `core/fm.rs`
/// became a directory; and `additive`'s own Nyquist helper was folded into a
/// shared `core::nyquist` module, which means the *previous*
/// source was edited by this change. So the probes had two jobs, not one: show
/// that nothing can name `fm4`, and show that `additive` still renders exactly
/// what it rendered yesterday.
///
/// 34 probes against `9e73ead`, all byte-identical. Twenty-one cover the
/// sources that predate `additive`, both FM ratio kinds, curved envelopes
/// either way, a pitch envelope, all three LFO targets, both filter kinds with
/// a velocity routing and a negative envelope amount, every effect, and a
/// chain of four. Four are `additive` alone, deliberately placed to work the
/// helper that moved: a sixteen-partial series played low enough to carry all
/// of it, high enough to lose most of it, and at the top of the keyboard where
/// almost nothing survives. Nine are songs — chords, step strings, a key with
/// degrees and a diatonic lift, swing and all three humanise axes, a stretch
/// `fit` with fades and an exact tail, a sidechained track chain beside a song
/// chain, a patch named by reference, and two additive songs, one of them
/// under a vibrato so the ceiling is decided by a moving pitch track rather
/// than a flat one.
///
/// Editing a source is normally exactly the case that bumps this number, and
/// the reason it does not here is worth being explicit about rather than
/// leaving to a reader to infer: the rule is *the samples move*, and the four
/// `additive` probes exist precisely because that claim about the helper's
/// move is the one nobody should take on trust. This is the compressor's case
/// again — further into shipping code than the change before it, and so
/// measured instead of reasoned about.
///
/// **Noise colour and oscillator unison are a leave-it-alone case of a new
/// shape**: the first where the new thing is a *field on an existing variant*
/// rather than a variant of its own, which is the shape that usually does move
/// this number — so it was checked rather than argued. `Source::Noise` grew a
/// `color` and `Osc` grew `voices` and `spread`, each defaulting to what the
/// crate already did and each skipped on the way out. 37 probes against
/// `eecdfcf` — all six sources, a stack of four, an `fm4` routing with
/// per-operator envelopes and a velocity index, an additive series both
/// harmonic and stretched, every envelope corner including both directions of
/// curve, a pitch envelope, the filter swept and velocity-routed, all three
/// LFO targets, every effect, a chain of four, and nine songs covering step
/// strings, chords, a key with degrees, swing, humanise, a track bus, a song
/// chain, a sidechain, panning, an automated fader and a drifting pan, `fit`,
/// `fade`, `tail` and an arrangement transform — came back byte-identical. A
/// recipe that names neither field renders what it always did, and the
/// defaults are what make that true by construction.
///
/// **Per-note articulation is that case again**, and it came out the same way
/// for the same reason: a mark is an optional field on each of the four ways an
/// entry is written, and a document written before it existed names none of
/// them, so every note in it is played exactly as written.
///
/// Checked against `0b4fd58` rather than argued from that sentence — 30 probes,
/// all byte-identical, encoded bytes and length both. All six sources,
/// including `fm4` with per-operator envelopes and a velocity index and both
/// coloured noises; a unison stack; every envelope corner and both directions
/// of curve; a pitch envelope; all three LFO targets; both filter kinds with a
/// velocity routing; every effect wet and dry, and a chain of several.
/// Twenty-one of them are songs, covering chords, step strings, keys and
/// degrees, swing, all three humanise axes, the arrangement transforms and the
/// track filter, a pan, a track bus, a song chain, a sidechain, an automated
/// cutoff and an automated fader, a fit, a fade and a tail.
///
/// The measure is the one this section keeps returning to, and it is worth
/// saying plainly for a notation: the note loop was edited — every note now
/// reads a mark before its gate, its velocity and its onset are decided — and
/// that is exactly why the probes were run rather than the sentence trusted. A
/// document that names no mark takes the identity through all three.
///
/// **Version 3 is the far end of that scale** and needs no argument at all: the
/// crate went stereo, so every bake in every project is a different file, of a
/// different channel count, and there is no recipe anywhere that renders to
/// what it used to.
///
/// **Version 4 is the master limiter changing what it limits against** — the
/// true peak rather than the loudest sample, under a ceiling of −1 dBTP rather
/// than 0.98. Every bake passes through it, so this is the *any representable
/// recipe* case at its widest: a recipe that used to touch the limiter comes
/// back about a decibel down, and one that never did comes back reconstructed
/// against a lower number and touches it now. There is no defaults argument to
/// have here and no probe worth running.
///
/// **Version 5 is `fm2`'s two operators starting where the note's seed says**
/// rather than both at zero — the last source that was still phase-locked, and
/// the leftover from the change that unlocked the oscillator stack. Every note
/// of every `fm2` patch is different samples, so this is the ordinary case of
/// the rule rather than an argument about defaults: there is no field to leave
/// unwritten and no recipe that opts out, because the phases were never in the
/// document.
///
/// No probe corpus, and the diff is why. The only edit outside `core/fm/two.rs`
/// is the call site handing it the seed it already had in hand — no shared
/// helper moved, no stage was reordered, and nothing any other source reads was
/// touched — so a recipe that does not name `fm2` cannot have moved, and one
/// that does moved by construction. Baking a corpus would be confirming a
/// sentence the diff already settles, which is the case this section names as
/// waste.
///
/// **Version 6 is a filter that modulates in octaves**, and it is the *any
/// representable recipe* case again: a cutoff is now
/// `cutoff × 2^(env_octaves × env + vel_octaves × vel + lfo)` where it used to
/// be `cutoff + env_amount × env + vel_cutoff × vel`, then multiplied by the
/// LFO. Every filtered patch that modulates at all comes back different, and
/// the ones that do not — no envelope depth, no velocity routing, no cutoff
/// LFO — come back identical, because a zero exponent is a multiply by one
/// exactly as a zero offset was an add of nothing.
///
/// That last sentence is the half worth checking rather than asserting, since
/// it is a change to the arithmetic and not a new optional field. Checked
/// against `d829817`, the commit before it — **33 probes, all
/// byte-identical**, encoded bytes and length both. Every source, including
/// `fm4` with per-operator envelopes, an additive series, a unison stack and
/// all three noise colours; a static lowpass and a static highpass, which read
/// a cutoff and modulate nothing; a pitch LFO and an amp LFO; a pitch
/// envelope; both signs of envelope curve; every effect on its own and a chain
/// of four; five songs covering swing with all three humanise axes, a song
/// chain, an automated fader and an automated pan; and five of the worked
/// examples out of `docs/recipes.md`.
///
/// The other half needs no probe: the old build has no field in which to write
/// the new depth, so a recipe that modulates its filter is a different
/// document on either side of this line rather than the same one rendered
/// twice.
///
/// **The same version carries the filter's two new modes and its slope**, and
/// neither would have earned a bump alone. [`FilterKind::Bandpass`](patch::FilterKind::Bandpass) and
/// [`FilterKind::Notch`](patch::FilterKind::Notch) are new words, so no document that exists can be
/// reinterpreted by them; [`Filter::slope`](patch::Filter::slope) is a new optional field defaulting
/// to the single pole pair every patch on disk already had, which is the case
/// this section says a diff answers without rendering anything. It was
/// re-checked anyway, because the pole pair is now reached through a cascade
/// rather than called directly and that is a stage reordered rather than a
/// field added: the same 33 probes against `2ccbd14`, all byte-identical
/// again.
pub const SYNTH_VERSION: u32 = 6;

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
