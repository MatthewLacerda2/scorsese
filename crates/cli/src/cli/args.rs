//! The arguments a subcommand takes, and what they mean to the rest of the crate.
//!
//! Split from the verbs themselves because they answer a different question. A
//! `Command` says *what the user asked for*; these say *what a word on the
//! command line stands for* — which asset kind, which starter, which action
//! within a group — and each carries the `From` that turns it into the type the
//! library actually speaks. Keeping them apart keeps the list of verbs readable
//! as a list of verbs.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use scorsese_core::style::Platform;
use scorsese_core::{AssetKind, Fps};
use scorsese_providers::synth::{Drum, Span, Starter};

/// What `new` takes.
#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// Where to create it, e.g. `teaser.scor`.
    pub(crate) directory: PathBuf,
    /// Project name. Defaults to the directory's name.
    #[arg(long)]
    pub(crate) name: Option<String>,
    /// The timeline framerate every clip and keyframe time is counted
    /// in: `30`, or a rational like `30000/1001` for 29.97. Chosen once,
    /// here — changing it later is a real operation, not a field edit.
    #[arg(long, default_value = "30")]
    pub(crate) fps: Fps,
    /// The placement the video is made for: `youtube`, `youtube_shorts`,
    /// `instagram_reels`, `instagram_reels_ad`, `instagram_stories_ad`,
    /// `tiktok` or `tiktok_ad`. Written into the brief with the render
    /// preset it means (`render --platform`); the project stores no platform.
    #[arg(long)]
    pub(crate) platform: Option<Platform>,
    /// The kind of video, by id: `kinetic_type`, `whiteboard`… An unknown id,
    /// or one not made for `--platform`, is refused with the ones that fit.
    /// Its prompt is written into the brief.
    #[arg(long)]
    pub(crate) style: Option<String>,
}

/// The things `sequence` does: bring a folder of frames in as one, or make
/// or change one from stills already in the pool.
#[derive(Debug, Subcommand)]
pub(crate) enum SequenceAction {
    /// Import a folder of frames as one image sequence: each frame becomes an
    /// image asset under `assets/<name>/`, and one `image_sequence` asset
    /// plays them.
    ///
    /// Frames play in the order their numbers say — `frame_9` before
    /// `frame_10` — and a gap in the numbering is reported, never refused.
    /// Only png, jpeg, bmp, tiff and webp count as frames; anything else in
    /// the folder is skipped and named. Frames of two formats or two sizes
    /// refuse the whole folder with nothing copied. Does not recurse.
    Import {
        /// The folder of frames. Copied in, never referenced in place.
        dir: PathBuf,
        /// How many timeline frames each still is held. Defaults to 1: a
        /// rendered frame directory or a timelapse, one photo a frame.
        #[arg(long)]
        hold: Option<u64>,
        /// Start again from the first still when it runs out, instead of
        /// holding the last one.
        #[arg(long = "loop")]
        looping: bool,
    },
    /// Make an image sequence from stills already in the pool, or change one:
    /// its stills, its hold, whether it loops.
    ///
    /// An asset id nothing answers to is made, from `--stills`. One that is a
    /// sequence is changed, and only in what is named. Nothing is written
    /// unless the project still validates.
    Set {
        /// The sequence's asset id.
        asset: String,
        /// The image assets it plays, in order, comma-separated — replacing
        /// the whole list. Required to make a new sequence.
        #[arg(long, value_delimiter = ',')]
        stills: Option<Vec<String>>,
        /// How many timeline frames each still is held.
        #[arg(long)]
        hold: Option<u64>,
        /// `true` to loop, `false` to hold the last still past the end.
        #[arg(long = "loop")]
        looping: Option<bool>,
    },
}

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
        /// Start from a library instrument instead — `kick`, `epiano`; `synth
        /// kit` lists them. The recipe is one note of it, its patch copied in
        /// for you to edit. Takes the place of `--kind`.
        #[arg(long, value_name = "NAME")]
        instrument: Option<String>,
    },
    /// List the library of ready-made instruments — a drum machine's kick,
    /// snare, closed hat and crash, a synth bass, a clav, a brass section, a
    /// pad and an electric piano — or print one's patch.
    ///
    /// A song track uses one as `"patch": "kit:kick"`, and `--copy-into`
    /// replaces each such name in a recipe with the patch itself: the song
    /// keeps its own copy, to edit freely, and never changes sound when
    /// scorsese is upgraded. A bake refuses a name that was never copied in.
    Kit {
        /// One instrument to print, as `kick` or `kit:kick`. Without it, the
        /// whole library is listed.
        #[arg(conflicts_with = "copy_into")]
        instrument: Option<String>,
        /// A recipe file to copy every `kit:` name into, rewritten in place.
        /// A relative path is relative to the project, as `recipes/song.json`.
        #[arg(long, value_name = "RECIPE")]
        copy_into: Option<PathBuf>,
    },
    /// Read a Standard MIDI File into a song recipe in `recipes/`, and add
    /// the asset that points at it — the way `new` does, from a `.mid`
    /// instead of a starter.
    ///
    /// The file's structure comes across as written: one song track per MIDI
    /// track and channel, channel 10 as a drum part, the tempo map, the key
    /// signature, every note with its velocity, cut into patterns of eight
    /// bars. Nothing is interpreted — no hands split, no repeats found — and
    /// every track plays a plain placeholder patch. What the song cannot hold
    /// (the sustain pedal, pitch bends, program numbers) is listed, not
    /// dropped silently.
    Import {
        /// The `.mid` file to read. It is not copied into the project: the
        /// recipe it becomes is what the project keeps.
        file: PathBuf,
        /// What to call the asset and its recipe. Defaults to the file's name
        /// without `.mid`, suffixed if that is taken.
        #[arg(long)]
        name: Option<String>,
    },
    /// Write a song recipe out as a Standard MIDI File, to open in a DAW —
    /// `import` the other way round.
    ///
    /// What is written is what the song plays: the arrangement once, with its
    /// transposes and mutes, chords and step strings as their notes, swing and
    /// articulations applied, and the tempo map — a ramp as a step every
    /// sixteenth note, since MIDI has only jumps. One MIDI track per song
    /// track, each on a channel of its own. The sounds are not written: the
    /// file is the score. Humanize, `fit`, glides and microtonal pitches are
    /// named in the report rather than dropped silently.
    Export {
        /// The synth_audio asset whose song to write.
        asset: String,
        /// Where to write the file. Without it, it lands in
        /// `cache/midi/<asset>.mid` — rebuildable from the recipe at any time.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        /// Put this track on channel 10, General MIDI's drum kit: `snare`
        /// keeps each note's key, `kick=36` plays every note of the track on
        /// that one key (36 kick, 38 snare, 42 closed hat). Repeat it for
        /// several. A song cannot say which of its tracks are drums, so
        /// nothing goes there unless named.
        #[arg(long = "drum", value_name = "TRACK[=KEY]")]
        drums: Vec<Drum>,
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

/// The things `stock` does: find free stock media, then bring it in.
#[derive(Debug, Subcommand)]
pub(crate) enum StockAction {
    /// Search Pixabay's free stock footage and photos — or, with `--lottie`,
    /// LottieFiles' free animations — five results a page, and write a
    /// contact sheet of their previews to look at before choosing.
    ///
    /// Free — neither library has a paid tier — so for a generic shot (a city
    /// at night, a sunrise, an office) it comes before `generate`, and for a
    /// mascot, an animated icon or illustration a Lottie usually beats
    /// drawing one. Results are cached in the project for 24 hours.
    Search {
        /// What to find, in plain words: `city at night`, `cat asleep`.
        #[arg(required_unless_present = "look", num_args = 1..)]
        words: Vec<String>,
        /// Search photos and illustrations instead of footage.
        #[arg(long, conflicts_with = "lottie")]
        image: bool,
        /// Search LottieFiles' animations instead of footage: characters,
        /// mascots, animated icons, which a page plays. Needs no key.
        #[arg(long, conflicts_with = "style")]
        lottie: bool,
        /// Only results this way round: `horizontal` or `vertical`. Footage and
        /// animations are filtered by their measured size.
        #[arg(long)]
        orientation: Option<String>,
        /// `film` or `animation` for footage; `photo`, `illustration` or
        /// `vector` with `--image`.
        #[arg(long)]
        style: Option<String>,
        /// Only footage, or animations, at least this many seconds long.
        #[arg(long)]
        min_seconds: Option<u32>,
        /// Which page of results, from 1.
        #[arg(long, default_value = "1")]
        page: u32,
        /// Include results Pixabay does not mark as suitable for all ages.
        #[arg(long)]
        unsafe_results: bool,
        /// Instead of searching, look through this video (or, with
        /// `--lottie`, this animation): five frames across the whole of it.
        /// Nothing is imported.
        #[arg(long, value_name = "ID", conflicts_with = "image")]
        look: Option<u64>,
        /// Where to write the sheet. Default: `cache/stock/sheet.png` in the
        /// project.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Download results a search found into `assets/`, as ordinary video or
    /// image assets named `pixabay-<id>` — or, with `--lottie`, animations
    /// into `pages/lottie-<id>.json`, for a page to play.
    ///
    /// Free. The file is the smallest Pixabay has that fills the render size
    /// without being enlarged, or the largest there is. A Lottie is not an
    /// asset: an html page loads it with the shipped lottie-web
    /// (`scorsese guide pages`).
    Import {
        /// The ids a search named. Several import at once; one that fails
        /// costs none of the others.
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The ids are photos, not footage.
        #[arg(long, conflicts_with = "lottie")]
        image: bool,
        /// The ids are LottieFiles animations, not footage.
        #[arg(long)]
        lottie: bool,
        /// The size the video will be rendered at.
        #[arg(long, default_value = "1920x1080")]
        resolution: scorsese_render::Resolution,
    },
}
