//! Cutting a narrated video to its voice.

use schemars::JsonSchema;
use scorsese_core::voice::{Measured, Scene, Voiced, Voicing, cut_to_voice};
use scorsese_core::{ClipId, Fps, Frames};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Lay a narrated cut out from its lines.
pub(crate) struct CutToVoice;

/// What `cut_to_voice` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The scenes, in the order they play. Each pairs one narration clip with
    /// the visual clips seen while it is said.
    scenes: Vec<SceneArgument>,
    /// Seconds from a line's last word to the end of its scene, where the
    /// next scene begins. Default 0.2.
    gap_seconds: Option<f64>,
    /// Seconds from a scene's start to its line's start, for every scene that
    /// does not set its own. Default 0.
    lead_in_seconds: Option<f64>,
    /// Seconds each scene's visuals run on under the next scene, so an exit
    /// and an entrance play together. Default 0, a straight cut. The incoming
    /// visual moves to another track to make room, and the reply says which.
    overlap_seconds: Option<f64>,
    /// Where the first scene begins, in seconds. Default: where its earliest
    /// visual starts now.
    start_seconds: Option<f64>,
}

/// One scene of `cut_to_voice`.
#[derive(Deserialize, JsonSchema)]
struct SceneArgument {
    /// Id of the narration clip: the line this scene is cut to.
    line: String,
    /// Ids of the visual clips — a page, a shot, a title. Each one runs the
    /// whole scene; footage that would run past the end of its file is
    /// refused.
    visuals: Vec<String>,
    /// Ids of clips that keep their offset from the scene's start — its sound
    /// effects, an overlay arriving part-way. Clips not named anywhere stay
    /// where they are.
    moves_with: Option<Vec<String>>,
    /// This scene's lead-in, in place of `lead_in_seconds`.
    lead_in_seconds: Option<f64>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("scenes", "the scenes in order, each a line and its visuals")];
}

impl Tool for CutToVoice {
    fn name(&self) -> &'static str {
        "cut_to_voice"
    }

    fn description(&self) -> &'static str {
        "Cut a narrated video to its voice: each scene ends a gap after its \
         line's last word, and the next begins there. Name the scenes in order, \
         each a narration clip and the visual clips that illustrate it. The \
         end is measured from the word timings kept with a generated line, or \
         from the end of the audio without them, and the reply says which. \
         Writes clip starts, durations and tracks only — re-run it after a line \
         is regenerated or re-worded. Clips listed under a scene move with it; \
         clips named nowhere stay put, and the reply lists those whose scene \
         changed. Nothing is changed unless the whole cut lands in a document \
         that loads."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir().to_owned();
        let dir = dir.as_path();
        let mut project = load(dir)?;
        let voicing = voicing(arguments);
        let voiced = cut_to_voice(&mut project, dir, &voicing)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(said(project.timeline_fps, &voiced).into())
    }
}

fn voicing(arguments: Arguments) -> Voicing {
    let ids = |ids: Vec<String>| ids.into_iter().map(ClipId::new).collect();
    Voicing {
        scenes: arguments
            .scenes
            .into_iter()
            .map(|scene| Scene {
                line: ClipId::new(scene.line),
                visuals: ids(scene.visuals),
                riders: ids(scene.moves_with.unwrap_or_default()),
                lead_in: scene.lead_in_seconds,
            })
            .collect(),
        lead_in: arguments.lead_in_seconds.unwrap_or(0.0),
        gap: arguments.gap_seconds.unwrap_or(0.2),
        overlap: arguments.overlap_seconds.unwrap_or(0.0),
        from: arguments.start_seconds,
    }
}

/// The cut, scene by scene, and everything it did that was not asked for in
/// so many words.
fn said(fps: Fps, voiced: &Voiced) -> String {
    let moment = |frames: Frames| format!("{:.2}s", fps.seconds(frames));
    let end = voiced.scenes.last().map_or(Frames::ZERO, |laid| laid.end);
    let mut lines = vec![format!(
        "{} scene(s) cut to the voice: the cut now ends at {} (was {}).",
        voiced.scenes.len(),
        moment(end),
        moment(voiced.was)
    )];
    for laid in &voiced.scenes {
        let from = match laid.measured {
            Measured::LastWord => "its last word",
            Measured::EndOfAudio => "the end of its audio (no word timings)",
        };
        lines.push(format!(
            "`{}`: {} to {}, ended from {from}",
            laid.line,
            moment(laid.start),
            moment(laid.end)
        ));
    }
    for (clip, track, made) in &voiced.rearranged {
        let made = if *made { " (new)" } else { "" };
        lines.push(format!(
            "`{clip}` moved to track `{track}`{made} to make room"
        ));
    }
    let list = |ids: &[ClipId]| {
        let names: Vec<String> = ids.iter().map(|id| format!("`{id}`")).collect();
        names.join(", ")
    };
    if !voiced.crossed.is_empty() {
        lines.push(format!(
            "Not named, so left where they were, but now under different \
             scenes: {} — move them, or name them under a scene's `moves_with`.",
            list(&voiced.crossed)
        ));
    }
    if !voiced.keyed_past_end.is_empty() {
        lines.push(format!(
            "Keyframes past their new end, written for the old length: {}.",
            list(&voiced.keyed_past_end)
        ));
    }
    lines.extend(
        voiced
            .follow_ups
            .iter()
            .map(|follow| format!("Next: {follow}.")),
    );
    lines.join("\n")
}
