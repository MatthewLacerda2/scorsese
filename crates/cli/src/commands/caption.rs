//! `scorsese caption`

use std::path::Path;

use anyhow::Result;
use scorsese_core::captions::{Captioned, Chunking, caption};
use scorsese_core::{FontChoice, Rgba, TrackId};
use scorsese_render::picture::caption as look;

/// What `scorsese caption` takes. Every look has a default, so a bare
/// `scorsese caption` is the whole request.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// The video track the captions go on. Made, on top, when there is none.
    #[arg(long, default_value = "captions")]
    track: String,
    /// A track whose clips are captioned. Repeatable; without it, every
    /// generated narration line on an audio track is, and every transcribed
    /// video or sound on any track.
    #[arg(long)]
    narration: Vec<String>,
    /// The most characters one caption holds — about two lines at the
    /// default size in 9:16.
    #[arg(long, default_value_t = Chunking::DEFAULT.max_chars)]
    max_chars: usize,
    /// The face: a name this build ships, or a font file inside the project.
    /// Montserrat when not given.
    #[arg(long)]
    font: Option<String>,
    /// How heavy, 1 to 1000. 800 when not given.
    #[arg(long)]
    weight: Option<u16>,
    /// How big, as a fraction of the frame's height. 0.031 when not given.
    #[arg(long)]
    size: Option<f64>,
    /// The words' colour, `#rrggbb` or `#rrggbbaa`. White when not given.
    #[arg(long)]
    color: Option<Rgba>,
    /// The rim outside each letter, `#rrggbb` or `#rrggbbaa`, or `none`.
    /// Black when not given.
    #[arg(long, value_name = "COLOUR|none")]
    stroke: Option<look::Stroke>,
    /// Leave out the rim outside each letter: `--stroke none`.
    #[arg(long, conflicts_with = "stroke")]
    no_stroke: bool,
    /// How far up from the frame's bottom edge, as a fraction of its height.
    /// 0.2, the lower third, when not given.
    #[arg(long)]
    lift: Option<f64>,
}

/// Writes the narration's words as captions on one video track.
///
/// Safe to re-run: it replaces only the captions it wrote, so running it again
/// after a line is regenerated or moved re-times them.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let mut project = super::open(project_dir)?;
    let asked = look::Choices {
        narration: options.narration.iter().map(TrackId::new).collect(),
        max_chars: Some(options.max_chars),
        font: options.font.clone().map(FontChoice::from),
        weight: options.weight,
        size: options.size,
        color: options.color,
        stroke: options
            .stroke
            .or(options.no_stroke.then_some(look::Stroke(None))),
        lift: options.lift,
    }
    .captioning(
        TrackId::new(options.track.as_str()),
        project.timeline_fps.as_f64(),
    );

    let report = caption(&mut project, project_dir, &asked)?;
    super::save(&project, project_dir)?;
    describe(&report, &asked.track);
    Ok(())
}

fn describe(report: &Captioned, track: &TrackId) {
    for (line, n) in &report.lines {
        println!("{line} — {n} caption(s)");
    }
    for line in &report.untimed {
        println!("{line} — skipped, no word timings");
    }
    let made: usize = report.lines.iter().map(|(_, n)| n).sum();
    println!(
        "{made} caption(s) on track `{track}`, {} replaced — ordinary text clips, yours to edit",
        report.replaced
    );
}
