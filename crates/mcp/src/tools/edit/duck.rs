//! Lowering the music while narration plays.

use schemars::JsonSchema;
use scorsese_core::{Dip, PropertyPath, TrackId, Under, duck_track};
use scorsese_render::audio::path::VOLUME;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
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
    /// Track ids that count as narration. Omit and every other audio track
    /// does.
    under: Option<Vec<String>>,
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
         the clips are rather than on the sound."
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

        let fps = project.timeline_fps.as_f64();
        let dip = Dip::from_seconds(
            arguments.depth.unwrap_or(Dip::UNDER),
            arguments.attack_seconds.unwrap_or(Dip::ATTACK_SECONDS),
            arguments.release_seconds.unwrap_or(Dip::RELEASE_SECONDS),
            fps,
        );
        let under = Under {
            music: TrackId::new(music),
            narration: arguments
                .under
                .iter()
                .flatten()
                .map(|id| TrackId::new(id.as_str()))
                .collect(),
        };

        let report = duck_track(&mut project, &under, &PropertyPath::new(VOLUME), dip)
            .ok_or_else(|| format!("no track `{music}` in this project"))?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!(
            "{} clip(s) ducked, {} left alone — ordinary volume keyframes, \
             editable and deletable",
            report.dipped.len(),
            report.untouched.len()
        )
        .into())
    }
}
