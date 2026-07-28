//! The command-line surface.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use scorsese_core::{AssetKind, Fps};
use scorsese_providers::Starter;
use scorsese_render::{
    AudioCodec, Bitrate, Container, Cue, FrameRange, Resolution, SampleRate, VideoCodec,
};

/// The whole command line: one verb, plus the options that outlive the choice
/// of verb. `about` is set explicitly rather than taken from this doc, so the
/// help a person reads and the doc a reader of the code reads can differ.
#[derive(Debug, Parser)]
#[command(
    name = "scorsese",
    version,
    about = "A video editor for agentic workflows"
)]
pub struct Cli {
    /// The verb, and everything that only that verb takes.
    #[command(subcommand)]
    pub command: Command,

    /// The project directory to work in. Defaults to the current directory.
    /// Global, so it can follow any subcommand: `scorsese assets gc
    /// --project teaser.scor`.
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,
}

impl Cli {
    /// `--project` if it was given, the current directory otherwise — which is
    /// what makes `cd teaser.scor && scorsese check` the short form of
    /// everything here.
    pub fn project_dir(&self) -> PathBuf {
        self.project.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Everything the binary can be asked to do. Each variant is dispatched to the
/// `commands` module of the same name, which is the whole of the CLI's own
/// logic — the rest lives in the library crates.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new project directory.
    New {
        /// Where to create it, e.g. `teaser.scor`.
        directory: PathBuf,
        /// Project name. Defaults to the directory's name.
        #[arg(long)]
        name: Option<String>,
        /// The timeline framerate every clip and keyframe time is counted
        /// in: `30`, or a rational like `30000/1001` for 29.97. Chosen once,
        /// here — changing it later is a real operation, not a field edit.
        #[arg(long, default_value = "30")]
        fps: Fps,
    },
    /// Copy a media file into the project and add it to the assets table.
    Import {
        /// The file to import. It is copied, never referenced in place.
        file: PathBuf,
        /// Override the kind instead of inferring it from the extension.
        #[arg(long, value_enum)]
        kind: Option<KindArg>,
    },
    /// Report everything wrong or questionable about the project — the
    /// document and the media it references — without rendering. Problems
    /// fail; warnings do not.
    Check {
        /// Re-hash every file to catch media that changed since import.
        /// Existence is always checked; hashing a whole pool costs real I/O
        /// and only ever produces warnings, so it is asked for.
        #[arg(long)]
        verify: bool,
    },
    /// Render the timeline to a video file.
    Render {
        /// Where to write the encoded file, e.g. `teaser.mp4`.
        #[arg(long)]
        out: PathBuf,
        /// Output resolution. Sources of a different shape meet it the way
        /// each clip's `fit` says — letterboxed, cropped, or left at their own
        /// size — and are never stretched.
        #[arg(long, default_value = "1920x1080")]
        resolution: Resolution,
        /// Output framerate. Defaults to the project's timeline framerate;
        /// anything else is conformed from it, nearest frame.
        #[arg(long)]
        fps: Option<Fps>,
        /// Target video bitrate, e.g. `8M`. Without it the encoder aims for
        /// constant quality instead.
        #[arg(long)]
        bitrate: Option<Bitrate>,
        /// Sample rate the mix is produced at, e.g. `48000` or `48k`. Sources
        /// recorded at other rates are resampled to it.
        #[arg(long, default_value = "48000")]
        sample_rate: SampleRate,
        /// Target audio bitrate, e.g. `192k`. Without it the encoder uses its
        /// own default, which is already transparent for speech and music.
        #[arg(long)]
        audio_bitrate: Option<Bitrate>,
        /// Render only part of the timeline, in frames: `30:120` covers frames
        /// 30 up to 120, `30:` runs to the end, `:120` from the start.
        #[arg(long)]
        range: Option<FrameRange>,
        /// Container to deliver in: `mp4`, `mkv`, `avi`, or `wmv`. Defaults to
        /// what `--out`'s extension asks for, so naming the file is usually
        /// the whole of this decision.
        #[arg(long)]
        container: Option<Container>,
        /// Picture codec: `h264`, `mpeg4`, or `wmv2`. Defaults to what the
        /// container is written with — H.264 for mp4 and mkv, MPEG-4 Part 2
        /// for avi, WMV 8 for wmv. A pairing scorsese does not write is
        /// refused before anything is encoded.
        #[arg(long)]
        video_codec: Option<VideoCodec>,
        /// Sound codec: `aac`, `pcm_s16le`, or `wmav2`. Defaults, like
        /// `--video-codec`, to what the container is written with.
        #[arg(long)]
        audio_codec: Option<AudioCodec>,
        /// Also write PNG stills of the finished file into this directory, so
        /// the pixels can be looked at without watching the video. The
        /// description says what the edit claims; these are what it did.
        #[arg(long)]
        stills: Option<PathBuf>,
        /// Which instants to still, comma-separated: `2.5s` for a time, `75`
        /// for a timeline frame. Without it, every segment boundary — which is
        /// where a cut can be one frame wrong.
        #[arg(long, value_delimiter = ',', requires = "stills")]
        at: Vec<Cue>,
    },
    /// Say what the timeline contains — what is on screen when, on which
    /// track, at what fit, with what animated, and what is audible under it.
    /// No render, no ffmpeg, no cost.
    Describe {
        /// Describe the cut as it would be rendered at this framerate.
        /// Defaults to the project's own timeline framerate.
        #[arg(long)]
        fps: Option<Fps>,
        /// Describe only part of the timeline, in frames: `30:120` covers
        /// frames 30 up to 120, `30:` runs to the end, `:120` from the start.
        #[arg(long)]
        range: Option<FrameRange>,
    },
    /// Make sound from a recipe the project carries: an effect, or a score.
    /// No key, no network, no cost, and the same bytes every time.
    Synth {
        /// What to do. Without one, the pending recipes are baked.
        #[command(subcommand)]
        action: Option<SynthAction>,
    },
    /// Lower the music while narration plays, by writing ordinary volume
    /// keyframes. Safe to re-run: it replaces only its own work.
    Duck {
        /// The audio track to duck — the music.
        #[arg(long)]
        music: String,
        /// How far down, as a multiplier on the clip's own level: `0.25` is a
        /// quarter as loud.
        #[arg(long, default_value = "0.25")]
        depth: f64,
        /// Seconds to reach the ducked level. The dip is fully down by the
        /// moment the narration starts, not after it.
        #[arg(long, default_value = "0.3")]
        attack: f64,
        /// Seconds to come back up. Longer than the attack on purpose —
        /// returning early is audible as a lurch.
        #[arg(long, default_value = "0.6")]
        release: f64,
        /// Which tracks count as narration. Repeatable; without it, every
        /// other audio track does.
        #[arg(long)]
        under: Vec<String>,
    },
    /// List the media pool and the state of everything in it.
    Assets {
        /// What to do with the pool. Without one, it is listed.
        #[command(subcommand)]
        action: Option<AssetsAction>,
        /// Re-hash every file to detect media that changed since import.
        #[arg(long)]
        verify: bool,
    },
}

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
