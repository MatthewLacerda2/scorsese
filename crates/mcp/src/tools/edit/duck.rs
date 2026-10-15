//! Lowering the music while narration plays.

use schemars::JsonSchema;
use scorsese_core::{ClipId, Dip, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Lower the music under narration.
pub(crate) struct Duck;

/// What `duck_music` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the audio track to duck — the music.
    music: Name,
    // The descriptions are built from `Dip`'s constants so the number they
    // print is the number the fallback below uses.
    #[schemars(description = format!(
        "How far down, as a multiplier on the clip's own level. 0.5 is half as \
         loud. Default {}.",
        Dip::UNDER
    ))]
    depth: Option<f64>,
    #[schemars(description = format!(
        "Seconds to reach the ducked level. The dip is fully down by the moment \
         narration starts. Default {}.",
        Dip::ATTACK_SECONDS
    ))]
    attack_seconds: Option<f64>,
    #[schemars(description = format!(
        "Seconds to come back up. Longer than the attack on purpose — returning \
         early lurches. Default {}.",
        Dip::RELEASE_SECONDS
    ))]
    release_seconds: Option<f64>,
    #[schemars(description = format!(
        "Where the music rests between lines, and before the first and after \
         the last, as a multiplier on the clip's own level. Below 1 keeps a \
         loud bed from crowding the voice in the pauses too. Default {}.",
        Dip::BETWEEN
    ))]
    between: Option<f64>,
    /// Seconds of pause that stay ducked: a pause this long or shorter is held
    /// under, a longer one breathes back up to `between`. Omit for one
    /// release plus one attack. Set it short (0.1) for narration in short
    /// phrases, so the music comes up between them.
    min_gap_seconds: Option<f64>,
    /// Track ids that count as narration, of any kind. Omit this, `clips` and
    /// `ranges` and every other audio track does.
    under: Option<Vec<String>>,
    /// Clip ids that count as narration, on a track of any kind: a line
    /// spoken inside a video shot, where the rest of that track is not
    /// speech.
    clips: Option<Vec<String>>,
    /// Stretches of the timeline that count as narration with no clip at
    /// all, as `[from, to]` in seconds: the words a page reads, captions with
    /// no audio of their own. They stay put when clips move.
    ranges: Option<Vec<[f64; 2]>>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("music", "the track id to duck")];
}

impl Tool for Duck {
    fn name(&self) -> &'static str {
        "duck_music"
    }

    fn description(&self) -> &'static str {
        "Lower a music track while narration plays over it, by writing ordinary \
         volume keyframes on its clips. Safe to run repeatedly: it replaces only \
         the keyframes it wrote and never touches ones set by hand. Works on \
         narration that has not been generated yet, since it triggers on where \
         the clips are rather than on the sound. Narration is every other audio \
         track unless you name tracks, clips or time ranges; `between` sets \
         where the music rests outside the lines and `min_gap_seconds` how \
         long a pause must be for it to come back up."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let music = arguments.music.as_str();

        let grid = project.timeline_fps;
        let fps = grid.as_f64();
        let mut dip = Dip {
            over: arguments.between.unwrap_or(Dip::BETWEEN),
            ..Dip::from_seconds(
                arguments.depth.unwrap_or(Dip::UNDER),
                arguments.attack_seconds.unwrap_or(Dip::ATTACK_SECONDS),
                arguments.release_seconds.unwrap_or(Dip::RELEASE_SECONDS),
                fps,
            )
        };
        if let Some(seconds) = arguments.min_gap_seconds {
            dip = dip.with_min_gap_seconds(seconds, fps);
        }
        let ranges = arguments.ranges.iter().flatten().map(|&[from, to]| {
            if from.is_finite() && to > from {
                Ok(grid.frames(from)..grid.frames(to))
            } else {
                Err(format!("range [{from}, {to}] does not run forward in time"))
            }
        });
        let under = Under {
            music: TrackId::new(music),
            narration: ids(arguments.under.as_ref(), TrackId::new),
            clips: ids(arguments.clips.as_ref(), ClipId::new),
            ranges: ranges.collect::<Result<_, _>>()?,
        };

        let report = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip)
            .map_err(|error| error.to_string())?;
        save(&project, dir)?;
        Ok(format!(
            "{} clip(s) ducked, {} left alone — ordinary volume keyframes, \
             editable and deletable",
            report.dipped.len(),
            report.untouched.len()
        )
        .into())
    }
}

/// The ids an optional list names, as `make` builds them.
fn ids<T>(list: Option<&Vec<String>>, make: impl Fn(String) -> T) -> Vec<T> {
    list.into_iter().flatten().cloned().map(make).collect()
}
