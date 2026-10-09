//! The narration's words as on-screen captions.

use schemars::JsonSchema;
use scorsese_core::captions::{Captioned, Captioning, caption};
use scorsese_core::{FontChoice, Rgba, TrackId};
use scorsese_render::picture::caption as look;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
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
    /// Audio track ids whose lines are captioned. Omit and every generated
    /// narration line on any audio track is.
    narration: Option<Vec<String>>,
    /// The most characters one caption holds, about two lines at the default
    /// size in 9:16. Default 36.
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
         word timings saved when each line was generated. For viewers watching \
         muted: each line is cut into short pieces at its sentences, commas and \
         pauses, and each piece arrives whole on its first spoken word. Writes \
         ordinary text assets and clips on one video track. Safe to re-run after \
         a line is regenerated or moved: it replaces only the captions it wrote. \
         A line without word timings is named and skipped."
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
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(said(&report, &asked.track).into())
    }
}

/// The run the arguments ask for, defaults filled in.
fn asked(arguments: &Arguments, fps: f64) -> Result<Captioning, String> {
    let track = args::given(arguments.track.as_deref()).unwrap_or("captions");
    let mut asked = look::captioning(TrackId::new(track), fps);
    let style = &mut asked.style;
    if let Some(font) = args::given(arguments.font.as_deref()) {
        style.font = FontChoice::from(font.to_owned());
        // A file is whatever weight it is, and says so itself; naming one
        // beside a static file is refused.
        if style.font.file().is_some() {
            style.weight = None;
        }
    }
    style.weight = arguments.weight.or(style.weight);
    style.size = arguments.size.unwrap_or(style.size);
    if let Some(color) = args::given(arguments.color.as_deref()) {
        style.color = colour(color, "color")?;
    }
    match args::given(arguments.stroke.as_deref()) {
        Some("none") => style.stroke = None,
        Some(stroke) => style.stroke = Some(colour(stroke, "stroke")?),
        None => {}
    }
    asked.narration = arguments
        .narration
        .iter()
        .flatten()
        .map(|id| TrackId::new(id.as_str()))
        .collect();
    if let Some(max) = arguments.max_chars {
        asked.chunking.max_chars = max as usize;
    }
    asked.lift = arguments.lift.unwrap_or(asked.lift);
    Ok(asked)
}

fn colour(text: &str, key: &str) -> Result<Rgba, String> {
    text.parse()
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
             before timings were kept, or not generated speech)"
        ));
    }
    lines.join("\n")
}
