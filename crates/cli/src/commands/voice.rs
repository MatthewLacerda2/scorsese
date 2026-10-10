//! `scorsese cut-to-voice`

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use scorsese_core::voice::{Measured, Scene, Voiced, Voicing, cut_to_voice};
use scorsese_core::{ClipId, Project};

/// What `scorsese cut-to-voice` takes.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// One scene, in the order they play: `LINE=VISUAL[,VISUAL…]`, then
    /// optionally `+CLIP[,CLIP…]` for clips that move with it (its sound
    /// effects) and `@SECONDS` for its own lead-in. Repeat once per scene —
    /// `--scene vo-1=page-1 --scene vo-2=page-2,bg+whoosh@0.3`.
    #[arg(long = "scene", required = true)]
    scenes: Vec<SceneSpec>,
    /// Seconds from a line's last word to the end of its scene, where the
    /// next one begins.
    #[arg(long, default_value_t = 0.2)]
    gap: f64,
    /// Seconds from a scene's start to its line's start, for every scene
    /// without its own `@SECONDS`.
    #[arg(long, default_value_t = 0.0)]
    lead_in: f64,
    /// Seconds each scene's visuals run on under the next, so an exit and an
    /// entrance play together. The incoming visual moves to another track.
    #[arg(long, default_value_t = 0.0)]
    overlap: f64,
    /// Where the first scene begins, in seconds. Where its earliest visual
    /// starts now, when not given.
    #[arg(long)]
    start: Option<f64>,
}

/// One `--scene`, parsed.
#[derive(Debug, Clone)]
struct SceneSpec(Scene);

impl FromStr for SceneSpec {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let (rest, lead_in) = match text.split_once('@') {
            Some((rest, lead)) => {
                let lead = lead
                    .parse()
                    .map_err(|_| format!("`{lead}` after `@` is not a number of seconds"))?;
                (rest, Some(lead))
            }
            None => (text, None),
        };
        let (rest, riders) = rest.split_once('+').unwrap_or((rest, ""));
        let (line, visuals) = rest
            .split_once('=')
            .ok_or_else(|| format!("`{text}` is not LINE=VISUAL[,VISUAL…]"))?;
        let ids = |list: &str| -> Vec<ClipId> {
            list.split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(ClipId::new)
                .collect()
        };
        Ok(Self(Scene {
            line: ClipId::new(line.trim()),
            visuals: ids(visuals),
            riders: ids(riders),
            lead_in,
        }))
    }
}

/// Lays the cut out from its lines and saves it.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let mut project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let voicing = Voicing {
        scenes: options.scenes.iter().map(|spec| spec.0.clone()).collect(),
        lead_in: options.lead_in,
        gap: options.gap,
        overlap: options.overlap,
        from: options.start,
    };
    let voiced = cut_to_voice(&mut project, project_dir, &voicing)
        .context("cutting to the voice — nothing was changed")?;
    project.save(project_dir).context("saving the project")?;
    describe(&project, &voiced);
    Ok(())
}

fn describe(project: &Project, voiced: &Voiced) {
    let fps = project.timeline_fps;
    for laid in &voiced.scenes {
        let from = match laid.measured {
            Measured::LastWord => "last word",
            Measured::EndOfAudio => "end of audio, no word timings",
        };
        println!(
            "{} — {:.2}s to {:.2}s, from its {from}",
            laid.line,
            fps.seconds(laid.start),
            fps.seconds(laid.end)
        );
    }
    for (clip, track, made) in &voiced.rearranged {
        let made = if *made { " (new)" } else { "" };
        println!("{clip} moved to track {track}{made} to make room");
    }
    for clip in &voiced.crossed {
        println!("{clip} — not named, left where it was, now under a different scene");
    }
    for clip in &voiced.keyed_past_end {
        println!("{clip} — keyframes past its new end");
    }
    for follow in &voiced.follow_ups {
        println!("next: {follow}");
    }
    let end = voiced
        .scenes
        .last()
        .map(|laid| laid.end)
        .unwrap_or_default();
    println!(
        "{} scene(s) cut to the voice: {:.2}s, was {:.2}s",
        voiced.scenes.len(),
        fps.seconds(end),
        fps.seconds(voiced.was)
    );
}
