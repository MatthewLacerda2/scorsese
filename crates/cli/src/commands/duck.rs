//! `scorsese duck`

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{Dip, Ducked, Project, PropertyPath, TrackId, Under, duck_track};
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
    /// Which tracks count as narration. Repeatable; without it, every
    /// other audio track does.
    #[arg(long)]
    under: Vec<String>,
}

/// Writes volume keyframes on the music track so it steps aside for narration.
///
/// Safe to re-run: it replaces only the tracks it signed, so a second run
/// redoes the ducking and leaves anything you set by hand exactly as it was.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let music = options.music.as_str();
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let fps = project.timeline_fps.as_f64();
    let dip = Dip::from_seconds(options.depth, options.attack, options.release, fps);
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
