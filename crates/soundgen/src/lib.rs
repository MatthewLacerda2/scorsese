//! # scorsese-soundgen — sound from a document
//!
//! Responsibility: turning a **recipe** — a synthesiser patch, or a song —
//! into samples. The oscillators, filters, envelopes and effects that make one
//! note; the tracker-style arrangement that makes a piece of music out of many;
//! and the WAV encoding that puts either into bytes.
//!
//! This is the free half of scorsese's generated content. A `synth_audio`
//! asset costs no money, needs no key and no network, and produces the **same
//! bytes every time**, which is what lets `generated/` address a bake by the
//! hash of the recipe that made it.
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
//! on any machine, on any run.
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
pub use note::{NoteOpts, midi_to_freq, midi_to_name, parse_note};
pub use patch::Patch;
pub use song::{PatchResolver, Song, render_song};

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
    let mixed = song::mix_song(song, resolve)?;
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
    /// up the room. Empty unless this was a song of more than one track; see
    /// [`song::Mixdown::tracks`].
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
