//! `scorsese caption`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::captions::{Captioned, caption};
use scorsese_core::{FontChoice, Project, Rgba, TrackId};
use scorsese_render::picture::caption as look;

/// What `scorsese caption` takes. Every look has a default, so a bare
/// `scorsese caption` is the whole request.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// The video track the captions go on. Made, on top, when there is none.
    #[arg(long, default_value = "captions")]
    track: String,
    /// An audio track whose lines are captioned. Repeatable; without it,
    /// every generated narration line on any audio track is.
    #[arg(long)]
    narration: Vec<String>,
    /// The most characters one caption holds — about two lines at the
    /// default size in 9:16.
    #[arg(long, default_value_t = 36)]
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
    /// Leave out the black rim outside each letter.
    #[arg(long)]
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
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let mut asked = look::captioning(
        TrackId::new(options.track.as_str()),
        project.timeline_fps.as_f64(),
    );
    asked.narration = options.narration.iter().map(TrackId::new).collect();
    asked.chunking.max_chars = options.max_chars;
    asked.lift = options.lift.unwrap_or(asked.lift);
    let style = &mut asked.style;
    if let Some(font) = &options.font {
        style.font = FontChoice::from(font.clone());
        if style.font.file().is_some() {
            style.weight = None;
        }
    }
    style.weight = options.weight.or(style.weight);
    style.size = options.size.unwrap_or(style.size);
    style.color = options.color.unwrap_or(style.color);
    if options.no_stroke {
        style.stroke = None;
    }

    let report = caption(&mut project, project_dir, &asked)?;
    project.save(project_dir).context("saving the project")?;
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
