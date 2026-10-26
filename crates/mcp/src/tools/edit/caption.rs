//! The narration's words as on-screen captions.

use std::fmt::Display;
use std::str::FromStr;

use schemars::JsonSchema;
use scorsese_core::captions::{Captioned, Captioning, Chunking, caption};
use scorsese_core::{FontChoice, PropertyPath, TrackId};
use scorsese_render::audio::path;
use scorsese_render::picture::caption as look;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Caption the narration.
pub(crate) struct CaptionNarration;

/// What `caption_narration` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the video track the captions go on. Made, on top, when there is
    /// none. Default `captions`.
    track: Option<String>,
    /// Track ids whose speaking clips (lines, videos, sounds) are captioned.
    /// Omit and every generated narration line on an audio track is, and
    /// every transcribed video or sound on any track unless its volume is 0
    /// throughout.
    narration: Option<Vec<String>>,
    #[schemars(description = format!(
        "The most characters one caption holds, about two lines at the default \
         size in 9:16. Default {}.",
        Chunking::DEFAULT.max_chars
    ))]
    max_chars: Option<u32>,
    /// The face: a name this build ships or a font file inside the project.
    /// Default `montserrat`.
    font: Option<String>,
    /// How heavy, 1 to 1000. Default 800.
    weight: Option<u16>,
    /// How big, as a fraction of the frame's height. Default 0.031.
    size: Option<f64>,
    /// The words' colour, `#rrggbb` or `#rrggbbaa`. Default white.
    color: Option<String>,
    /// The rim outside each letter, `#rrggbb`, or `none`. Default black.
    stroke: Option<String>,
    /// How far up from the frame's bottom edge, as a fraction of its height.
    /// Default 0.2, the lower third.
    lift: Option<f64>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[];
}

impl Tool for CaptionNarration {
    fn name(&self) -> &'static str {
        "caption_narration"
    }

    fn description(&self) -> &'static str {
        "Put the narration's own words on screen as captions, timed from the \
         word timings saved when each line was generated, and from footage's \
         speech once transcribe has heard it. For viewers watching muted: each \
         line is cut into short pieces at its sentences, commas and pauses, and \
         each piece arrives whole on its first spoken word. Writes ordinary text \
         assets and clips on one video track. Safe to re-run after a line is \
         regenerated or moved, or a transcript corrected: it replaces only the \
         captions it wrote. A clip without word timings on a track named in \
         `narration` is named and skipped."
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
        let asked = asked(&arguments, project.timeline_fps.as_f64())?;
        let report = caption(&mut project, dir, &asked).map_err(|error| error.to_string())?;
        save(&project, dir)?;
        Ok(said(&report, &asked.track).into())
    }
}

/// The run the arguments ask for, defaults filled in.
fn asked(arguments: &Arguments, fps: f64) -> Result<Captioning, String> {
    let track = args::given(arguments.track.as_deref()).unwrap_or("captions");
    let choices = look::Choices {
        narration: arguments
            .narration
            .iter()
            .flatten()
            .map(|id| TrackId::new(id.as_str()))
            .collect(),
        max_chars: arguments.max_chars.map(|max| max as usize),
        font: args::given(arguments.font.as_deref()).map(|font| FontChoice::from(font.to_owned())),
        weight: arguments.weight,
        size: arguments.size,
        color: parsed("color", arguments.color.as_deref())?,
        stroke: parsed("stroke", arguments.stroke.as_deref())?,
        lift: arguments.lift,
    };
    let mut asked = choices.captioning(TrackId::new(track), fps);
    asked.volume = Some(PropertyPath::new(path::VOLUME));
    Ok(asked)
}

/// `text` read as a `T`, when it was given at all.
fn parsed<T: FromStr>(key: &str, text: Option<&str>) -> Result<Option<T>, String>
where
    T::Err: Display,
{
    args::given(text)
        .map(str::parse)
        .transpose()
        .map_err(|problem| format!("`{key}`: {problem}"))
}

/// What the run reads as.
fn said(report: &Captioned, track: &TrackId) -> String {
    let made: usize = report.lines.iter().map(|(_, n)| n).sum();
    let mut lines = vec![format!(
        "{made} caption(s) from {} line(s) on track `{track}`{} — ordinary text \
         clips, editable and deletable",
        report.lines.len(),
        match report.replaced {
            0 => String::new(),
            n => format!(", replacing the {n} written before"),
        }
    )];
    for clip in &report.untimed {
        lines.push(format!(
            "{clip}: skipped — no word timings (not generated yet, generated \
             before timings were kept, or footage not transcribed yet)"
        ));
    }
    lines.join("\n")
}
