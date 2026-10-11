//! `scorsese duck`

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::{ClipId, Dip, Ducked, Project, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;

/// What `scorsese duck` takes: the music, and how the dip is shaped before it
/// is put on the project's frame grid.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// The audio track to duck — the music.
    #[arg(long)]
    music: String,
    /// How far down, as a multiplier on the clip's own level: `0.5` is half
    /// as loud.
    #[arg(long, default_value_t = Dip::UNDER)]
    depth: f64,
    /// Seconds to reach the ducked level. The dip is fully down by the
    /// moment the narration starts, not after it.
    #[arg(long, default_value_t = Dip::ATTACK_SECONDS)]
    attack: f64,
    /// Seconds to come back up. Longer than the attack on purpose —
    /// returning early is audible as a lurch.
    #[arg(long, default_value_t = Dip::RELEASE_SECONDS)]
    release: f64,
    /// Where the music rests between lines, and before the first and after
    /// the last, as a multiplier on the clip's own level. Below 1.0 keeps a
    /// loud bed from crowding the voice even in the pauses.
    #[arg(long, default_value_t = Dip::BETWEEN)]
    between: f64,
    /// Seconds of pause that stay ducked: a pause this long or shorter is held
    /// under, a longer one breathes back up to `--between`. Without it, one
    /// release plus one attack (0.9 s by default). A pause shorter than the
    /// two ramps shares itself between them.
    #[arg(long, value_name = "SECONDS")]
    min_gap: Option<f64>,
    /// Which tracks count as narration, of any kind. Repeatable. Without
    /// `--under`, `--clip` or `--range`, every other audio track does.
    #[arg(long)]
    under: Vec<String>,
    /// A clip that counts as narration, on a track of any kind: a line spoken
    /// inside a video shot. Repeatable.
    #[arg(long, value_name = "CLIP")]
    clip: Vec<String>,
    /// A stretch of the timeline, in seconds, that counts as narration with
    /// no clip at all: `12.5:15`. Repeatable. It stays where it was put when
    /// clips move.
    #[arg(long, value_name = "FROM:TO", value_parser = seconds_range)]
    range: Vec<(f64, f64)>,
}

/// `FROM:TO` in seconds, both ends required and in order.
fn seconds_range(text: &str) -> Result<(f64, f64), String> {
    let (from, to) = text
        .split_once(':')
        .ok_or_else(|| format!("`{text}` is not FROM:TO, in seconds"))?;
    let end = |part: &str| {
        part.trim()
            .parse::<f64>()
            .ok()
            .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
            .ok_or_else(|| format!("`{part}` in `{text}` is not a time in seconds"))
    };
    let (from, to) = (end(from)?, end(to)?);
    if to <= from {
        return Err(format!("`{text}` ends before it starts"));
    }
    Ok((from, to))
}

/// Writes volume keyframes on the music track so it steps aside for narration.
///
/// Safe to re-run: it replaces only the tracks it signed, so a second run
/// redoes the ducking and leaves anything you set by hand exactly as it was.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let music = options.music.as_str();
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let grid = project.timeline_fps;
    let fps = grid.as_f64();
    let mut dip = Dip {
        over: options.between,
        ..Dip::from_seconds(options.depth, options.attack, options.release, fps)
    };
    if let Some(seconds) = options.min_gap {
        dip = dip.with_min_gap_seconds(seconds, fps);
    }
    let under = Under {
        music: TrackId::new(music),
        narration: options.under.iter().map(TrackId::new).collect(),
        clips: options.clip.iter().map(ClipId::new).collect(),
        ranges: options
            .range
            .iter()
            .map(|(from, to)| grid.frames(*from)..grid.frames(*to))
            .collect(),
    };

    // The property is named here rather than in `scorsese-core`, which defines
    // property types and never property values. `volume` belongs to the mixer
    // that implements it, and this is the seam where the two meet.
    let report = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip)?;

    if report.is_empty() {
        println!("track `{music}` has no clips on it");
        return Ok(());
    }
    project.save(project_dir).context("saving the project")?;
    describe(&report, options.depth);
    Ok(())
}

fn describe(report: &Ducked, depth: f64) {
    for clip in &report.dipped {
        println!("{clip} — ducked to {depth:.2} under narration");
    }
    for clip in &report.untouched {
        println!("{clip} — nothing over it, left alone");
    }
    println!(
        "{} ducked, {} untouched — ordinary volume keyframes, yours to edit",
        report.dipped.len(),
        report.untouched.len()
    );
}
