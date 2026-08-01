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
use scorsese_providers::synth::Starter;

/// The things `synth` does. Baking is the default, so the common case needs
/// no verb at all.
#[derive(Debug, Subcommand)]
pub enum SynthAction {
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
    Bake {
        /// Bake only this asset. Without it, every synth asset is considered.
        asset: Option<String>,
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
pub enum StarterArg {
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
pub enum AssetsAction {
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
pub enum KindArg {
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
