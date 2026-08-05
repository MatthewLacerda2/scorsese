//! The command-line surface.
//!
//! `Command` is the whole of what the binary does — one variant per verb, each
//! carrying only what that verb takes. The second subcommand level, and the
//! enumerated values a flag accepts, are `actions`: kept apart because this
//! file answers "what can scorsese be asked to do?" and that one answers "what
//! words may an answer be spelled with?".

use std::path::PathBuf;

mod args;

pub(crate) use args::{AssetsAction, KindArg, SynthAction};

use clap::{Parser, Subcommand};
use scorsese_core::Fps;
use scorsese_render::contact;
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
    pub(crate) command: Command,

    /// The project directory to work in. Defaults to the current directory.
    /// Global, so it can follow any subcommand: `scorsese assets gc
    /// --project teaser.scor`.
    #[arg(long, global = true)]
    pub(crate) project: Option<PathBuf>,
}

impl Cli {
    /// `--project` if it was given, the current directory otherwise — which is
    /// what makes `cd teaser.scor && scorsese check` the short form of
    /// everything here.
    pub(crate) fn project_dir(&self) -> PathBuf {
        self.project.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Everything the binary can be asked to do. Each variant is dispatched to the
/// `commands` module of the same name, which is the whole of the CLI's own
/// logic — the rest lives in the library crates.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Say where provider keys are kept on this machine, which of them
    /// resolved, and from where.
    ///
    /// Never prints a key — not masked, not truncated. What it answers is
    /// whether one was found and which of the two places had it, which is the
    /// question anybody debugging a missing credential actually has.
    Settings,
    /// List the ElevenLabs voices a narration can be read in, or check that
    /// one still exists.
    ///
    /// Costs nothing and spends nothing: a listing is free, and the answer is
    /// cached inside the project so browsing needs no network. There is no
    /// default voice and there never will be — every ElevenLabs default voice
    /// expires on 2026-12-31, so an id written into scorsese would be an
    /// outage with a date already on it.
    Voices {
        /// Search the Voice Library — thousands of voices other people
        /// published — instead of the small built-in set. Needs a paid
        /// ElevenLabs plan; the built-in voices do not.
        #[arg(long)]
        library: bool,
        /// Only voices in this language, as an ISO 639-1 code: `pt`, `en`,
        /// `es`. The filter worth reaching for first — it is what surfaces
        /// people who actually speak the language rather than read it.
        #[arg(long, requires = "library")]
        language: Option<String>,
        /// Only voices in this regional variant, where the vendor has one:
        /// `pt-br`, `en-us`. Finer than `--language`, and not every voice
        /// carries it.
        #[arg(long, requires = "library")]
        locale: Option<String>,
        /// Only voices of this gender: `male`, `female`, `neutral`.
        #[arg(long, requires = "library")]
        gender: Option<String>,
        /// Only voices of this age: `young`, `middle_aged`, `old`.
        #[arg(long, requires = "library")]
        age: Option<String>,
        /// Only voices with this accent, matched loosely: `british`,
        /// `brazilian`.
        #[arg(long, requires = "library")]
        accent: Option<String>,
        /// How many to return, at most 100. The vendor's own default is 30.
        #[arg(long, requires = "library")]
        page_size: Option<u32>,
        /// Read the list from ElevenLabs again even if the cached one is still
        /// current. The cache re-reads itself weekly on its own.
        #[arg(long, conflicts_with = "check")]
        refresh: bool,
        /// Ask about one voice id instead of listing anything: whether it still
        /// exists and can be generated with. Never answered from the cache —
        /// catching an id that has gone is the whole point of it.
        #[arg(long, conflicts_with = "library", value_name = "VOICE_ID")]
        check: Option<String>,
    },
    /// Print what a generated shot costs, per tier and per size, with the day
    /// each figure was last read off the vendor's page.
    ///
    /// Every figure is a published rate somebody copied, never a bill: no
    /// provider reports what a generation actually cost. Prints as Markdown,
    /// because CI appends this to a job summary.
    Prices,
    /// Realise every sketched brief: hand each to its provider, wait a while,
    /// and collect what finishes.
    ///
    /// The one command here that spends money. Shots go to Veo and narration
    /// to ElevenLabs; a key is asked for only by the half that has work, so a
    /// project of nothing but narration needs no Veo key. A brief whose file is
    /// already in generated/ is never sent again, so running this twice costs
    /// nothing the second time.
    ///
    /// Video takes minutes: whatever is still generating when the wait runs out
    /// is not lost — its ticket is in project.json, and `--collect` picks it
    /// up. Narration comes back on the same call and is never in flight. A line
    /// with no voice chosen yet is reported and skipped, not failed.
    Generate {
        /// Say what a run would cost and stop. Needs no key, sends nothing.
        #[arg(long)]
        dry_run: bool,
        /// Collect what has finished and submit nothing at all. What to run
        /// after coming back to a project with shots in flight.
        #[arg(long, conflicts_with = "dry_run")]
        collect: bool,
        /// How many seconds to wait before detaching, leaving the rest to be
        /// collected later. Most shots finish inside the default.
        #[arg(long, default_value_t = 300)]
        wait: u64,
    },
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
    /// Copy media into the project and add it to the assets table.
    ///
    /// A directory brings in the media directly inside it, one asset each,
    /// sorted by file name and without recursing — the directory itself never
    /// becomes an asset. Files that are not media are skipped and named, and a
    /// file whose id an asset already answers to is refused with nothing
    /// copied at all.
    Import {
        /// The file or directory to import. What comes in is copied, never
        /// referenced in place.
        path: PathBuf,
        /// Override the kind instead of inferring it from the extension. For a
        /// directory this says what the media in it is; which files count as
        /// media at all is still the extension's answer.
        #[arg(long, value_enum)]
        kind: Option<KindArg>,
    },
    /// Ask ffprobe about every asset that has a file and no recorded
    /// metadata, and write down what it says: how long the source is, how big
    /// it is, and whether it carries sound.
    ///
    /// Import already does this for what it brings in; this is for everything
    /// that reached `project.json` another way. Safe to re-run — an
    /// already-probed asset is left alone.
    Probe {
        /// Read every file again, replacing metadata that is already
        /// recorded. For when what is written down is wrong; without it, only
        /// the assets nobody has looked at are probed.
        #[arg(long)]
        all: bool,
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
    /// Write one frame as a PNG, composited exactly as a render would compose
    /// it — no encode, no video file, no sound. What `render --stills` costs a
    /// whole render to answer, this answers for one frame.
    Still {
        /// Which instant to compose, comma-separated: `9.1s` for a time, `285`
        /// for a timeline frame. Several instants write several files.
        #[arg(long, value_delimiter = ',', required = true)]
        at: Vec<Cue>,
        /// Where to write the PNG, e.g. `frame.png`. With several instants the
        /// frame number is added to the name: `frame-00285.png`.
        #[arg(long)]
        out: PathBuf,
        /// The raster to composite at, and so the size of the PNG. Everything
        /// a title or a layout is placed by is a fraction of the frame, so a
        /// smaller one is the same picture and costs less to make.
        #[arg(long, default_value = "1920x1080")]
        resolution: Resolution,
    },
    /// Look at a video file: frames sampled from it, tiled into one PNG with
    /// each frame's timestamp on it.
    ///
    /// A file, not a project — an imported asset or footage nobody has
    /// imported yet — so this answers "what is in this?" where `still`
    /// answers "what does my cut look like?".
    Look {
        /// The video file to look at.
        file: PathBuf,
        /// Where to write the PNG. Defaults to `<file>.sheet.png`, beside the
        /// source.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Where to start, in seconds. A file longer than one sheet is walked
        /// in successive calls, and each one says where the next starts.
        #[arg(long, default_value_t = 0.0)]
        from: f64,
        /// Where to stop, in seconds. Without it the frames are five seconds
        /// apart; with it they spread evenly across the span you named.
        #[arg(long)]
        to: Option<f64>,
        /// How many frames to take, at most five.
        #[arg(long, default_value_t = contact::MAX_FRAMES)]
        frames: usize,
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
    /// Dissolve one shot into the next, by writing ordinary opacity keyframes
    /// on both.
    ///
    /// Two clips on one track may not overlap and a crossover needs them to,
    /// so the incoming clip moves to a track above and is pulled back over the
    /// outgoing one. What that rearranged is printed rather than discovered.
    Dissolve {
        /// The outgoing clip — the shot being left.
        #[arg(long)]
        from: String,
        /// The incoming clip — the shot arriving. It ends up on a track above.
        #[arg(long)]
        to: String,
        /// How long the crossover lasts, in seconds.
        #[arg(long, default_value = "0.5")]
        seconds: f64,
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
    /// Say how a finished sound file came out — over its whole length, over
    /// time, and across the spectrum — and optionally how it differs from
    /// another. A signal, never a gate: there is no correct loudness, so
    /// nothing here can fail.
    Level {
        /// The file to measure. A bake, a render, or any media ffmpeg can
        /// decode sound out of.
        file: PathBuf,
        /// Compare against this file, field by field. The version this was
        /// meant to replace, usually — a difference is far easier to judge
        /// than an absolute number.
        #[arg(long)]
        against: Option<PathBuf>,
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
