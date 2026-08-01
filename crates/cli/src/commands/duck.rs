//! `scorsese duck`

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{Dip, Ducked, Frames, Project, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;

/// How the dip is shaped, before it is put on the project's frame grid.
pub(crate) struct Options {
    /// How far down, as a multiplier on the clip's own level.
    pub(crate) depth: f64,
    /// Seconds to get there.
    pub(crate) attack: f64,
    /// Seconds to come back.
    pub(crate) release: f64,
    /// Which tracks count as narration. Empty means every other audio track.
    pub(crate) under: Vec<String>,
}

/// Writes volume keyframes on the music track so it steps aside for narration.
///
/// Safe to re-run: it replaces only the tracks it signed, so a second run
/// redoes the ducking and leaves anything you set by hand exactly as it was.
pub(crate) fn run(project_dir: &Path, music: &str, options: &Options) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let fps = project.timeline_fps.as_f64();
    let dip = Dip {
        under: options.depth,
        over: 1.0,
        attack: seconds(options.attack, fps),
        release: seconds(options.release, fps),
    };
    let under = Under {
        music: TrackId::new(music),
        narration: options.under.iter().map(TrackId::new).collect(),
    };

    // The property is named here rather than in `scorsese-core`, which defines
    // property types and never property values. `volume` belongs to the mixer
    // that implements it, and this is the seam where the two meet.
    let Some(report) = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip) else {
        bail!("no track `{music}` in this project");
    };

    if report.is_empty() {
        println!("track `{music}` has no clips on it");
        return Ok(());
    }
    project.save(project_dir).context("saving the project")?;
    describe(&report, options.depth);
    Ok(())
}

/// Seconds on the project's own grid, at least one frame when anything was
/// asked for — a ramp rounded to zero frames is a switch, not a ramp.
fn seconds(seconds: f64, fps: f64) -> Frames {
    if seconds <= 0.0 {
        return Frames::ZERO;
    }
    Frames(((seconds * fps).round() as u64).max(1))
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
