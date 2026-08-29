//! The arguments a subcommand takes, and what they mean to the rest of the crate.
//!
//! Split from the verbs themselves because they answer a different question. A
//! `Command` says *what the user asked for*; these say *what a word on the
//! command line stands for* — which asset kind, which starter, which action
//! within a group — and each carries the `From` that turns it into the type the
//! library actually speaks. Keeping them apart keeps the list of verbs readable
//! as a list of verbs.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use scorsese_core::AssetKind;
use scorsese_providers::synth::{Span, Starter};

/// The things `synth` does. Baking is the default, so the common case needs
/// no verb at all.
#[derive(Debug, Subcommand)]
pub(crate) enum SynthAction {
    /// Write a starter recipe into `recipes/` and add the asset that points
    /// at it. The starter makes a sound as written, so the first bake is
    /// something to listen to rather than silence.
    New {
        /// What to call it. Becomes the asset id and the recipe's file name,
        /// suffixed if that name is taken.
        name: String,
        /// Which shape to start from: `patch` for one instrument and one note,
        /// `song` for an arrangement.
        #[arg(long, default_value = "patch")]
        kind: StarterArg,
    },
    /// Render the recipes that are not already baked, into `generated/`.
    /// Safe to re-run: an unchanged recipe is a cache hit.
    ///
    /// Given `--beats`, `--seconds` or `--only`, it bakes *less* of one
    /// recipe instead: a stretch of the piece, or a few of its tracks. That
    /// output is **not** cached — it goes to `cache/synth/`, or wherever
    /// `--out` says — because a fragment stored under the address of the whole
    /// recipe would leave the project holding audio its recipe does not
    /// describe.
    Bake {
        /// Bake only this asset. Without it, every synth asset is considered.
        asset: Option<String>,
        /// Render only these beats of the rendered piece: `0:32`, `16:`,
        /// `:32`. Beats, not bars — a song has no time signature, so eight
        /// bars of four is `0:32`. Counted along what is rendered, which under
        /// a `loop` fit is not the written arrangement. Not cached.
        #[arg(long, value_name = "FROM:TO", conflicts_with = "seconds")]
        beats: Option<Span>,
        /// The same window said in seconds of the rendered piece: `0:12`,
        /// `8:`, `:12`. Not cached.
        #[arg(long, value_name = "FROM:TO")]
        seconds: Option<Span>,
        /// Render only this track, by the name the song's notes use. Repeat it
        /// for several. The song's own fx and the master limiter still run, so
        /// what comes back is the mix with fewer parts in it rather than a
        /// bare instrument. Not cached.
        #[arg(long, value_name = "TRACK")]
        only: Vec<String>,
        /// Where to write a partial bake. Without it, one lands in
        /// `cache/synth/<asset>.wav` and the next overwrites it.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Parse a recipe and report what it is, without rendering it — so a
    /// malformed document costs milliseconds rather than a bake.
    Check {
        /// The recipe file to read.
        recipe: PathBuf,
    },
    /// Say what every song recipe in the project is made of, and count the
    /// same facts across them: which sources, at what gain and cutoff, at what
    /// tempo, in what register. Reads the documents only — no bake, no ffmpeg,
    /// no cost. A count and never a score: nothing here can fail anything.
    Survey,
}

/// Which starter `synth new` writes.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum StarterArg {
    /// One instrument, one note: the shape an effect takes.
    Patch,
    /// Four bars of one instrument: the shape a score takes.
    Song,
}

impl From<StarterArg> for Starter {
    fn from(arg: StarterArg) -> Self {
        match arg {
            StarterArg::Patch => Self::Patch,
            StarterArg::Song => Self::Song,
        }
    }
}

/// The things `assets` does beyond reporting what is in the pool.
#[derive(Debug, Subcommand)]
pub(crate) enum AssetsAction {
    /// Report assets no clip references, and optionally delete them.
    Gc {
        /// Actually remove them. Without this, nothing is deleted.
        #[arg(long)]
        delete: bool,
    },
}

/// The kinds a file can be imported as. The prompt-backed kinds are absent on
/// purpose: they are authored, not imported.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    /// Moving pictures, and whatever audio the file carries alongside them.
    Video,
    /// A still, which holds the screen for as long as its clip is on it.
    Image,
    /// Sound on its own — music, narration, an effect.
    Audio,
}

impl From<KindArg> for AssetKind {
    fn from(arg: KindArg) -> Self {
        match arg {
            KindArg::Video => Self::Video,
            KindArg::Image => Self::Image,
            KindArg::Audio => Self::Audio,
        }
    }
}
