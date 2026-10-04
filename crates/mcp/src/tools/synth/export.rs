//! A song recipe out as a MIDI file: `synth_export`.
//!
//! `synth_import` the other way round, and the door out to every other tool a
//! musician owns: a `.mid` opens in any DAW, which is where a user checks or
//! finishes zimmer's work with the tools they already know.

use schemars::JsonSchema;
use scorsese_core::AssetId;
use scorsese_providers::synth::{self, Drum};
use serde::Deserialize;
use serde_json::Value;

use super::strings;
use crate::tools::args::{self, Name, ProjectDir};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Write a song recipe as a `.mid`.
pub(crate) struct Export;

/// What `synth_export` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The synth_audio asset whose song recipe to write. A one-shot has no
    /// notes in time and is refused.
    asset: Name,
    /// Where to write the .mid — relative to the project directory, or
    /// absolute. Omit and it lands in cache/midi/<asset>.mid: rebuildable from
    /// the recipe, so it is not kept as an asset.
    out: Option<String>,
    /// Tracks to write on channel 10, General MIDI's drum kit, each as "track"
    /// to keep every note's key, or "track=key" to play every note of it on
    /// that one key: "kick=36", "snare=38", "hat=42". An imported file's drum
    /// part is already a kit, so it is named bare: "drums". Omit and every
    /// track is a pitched part.
    #[serde(default, deserialize_with = "strings")]
    #[schemars(with = "Vec<String>")]
    drums: Vec<String>,
}

impl args::Arguments for Arguments {}

impl Tool for Export {
    fn name(&self) -> &'static str {
        "synth_export"
    }

    fn description(&self) -> &'static str {
        "Write a song recipe out as a Standard MIDI File, to open in a DAW — \
         synth_import the other way round. What is written is what the song \
         plays: the arrangement once with its transposes and mutes, chords and \
         step strings as their notes, swing and articulations applied, and the \
         tempo map, a ramp as a step every sixteenth note since MIDI has only \
         jumps. One MIDI track per song track, each on its own channel, named \
         for it. The sounds are not written — the file is the score — and a \
         song cannot say which tracks are drums, so name them in `drums` to put \
         them on General MIDI's kit. The reply names what the file could not \
         carry (humanize, fit, glides, microtonal pitches). The project is not \
         changed. Costs nothing."
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
        let id = AssetId::new(arguments.asset.as_str());
        // Each entry read the way the command line's `--drum` is.
        let drums = arguments
            .drums
            .iter()
            .map(|entry| entry.parse::<Drum>())
            .collect::<Result<Vec<_>, _>>()?;
        let out = args::under(dir, arguments.out.as_deref(), "out")?;
        let project = load(dir)?;
        let mut exported = synth::export_midi(&project, dir, &id, &drums, out.as_deref())
            .map_err(|error| format!("{error}"))?;
        // Said back in the caller's own words, which resolve from the project
        // the way every path given to this server does.
        if let Some(given) = arguments.out.as_deref() {
            given.clone_into(&mut exported.shown);
        }
        let mut lines = vec![format!("{id} — written as MIDI to {}", exported.shown)];
        lines.extend(exported.lines());
        Ok(lines.join("\n").into())
    }
}
