//! What the whole set of recipes is made of, for a client that cannot hear it.
//!
//! Its own file rather than another entry beside `synth_bake`, because it
//! answers a different question from every other tool here: those all ask *how
//! did this one come out*, and this one asks *what are all of these*. The rest
//! of the loop — write, bake, listen, adjust — is one recipe at a time, and
//! nothing in it can notice that six of them are the same.

use scorsese_providers::synth;
use serde_json::Value;

use super::super::inspect::load;
use super::super::{Costs, Reply, Tool, project_dir, project_only_schema};

/// Count what every song recipe is made of.
pub(in crate::tools) struct Survey;

impl Tool for Survey {
    fn name(&self) -> &'static str {
        "synth_survey"
    }

    fn description(&self) -> &'static str {
        "Say what every song recipe in the project is made of, and count the \
         same facts across the whole set. Per song: the tempo it plays at, the \
         register its notes span, the pitch classes they fall in, and a row per \
         track. A track row says what the instrument is — source kind, gain, \
         where its filter starts — and then what it does in the piece: its \
         envelope's sustain, how many notes a second it plays, and the median \
         pitch it sits at. That second half is the half a source kind does not \
         predict: three cues on three different generators can share a sustain \
         of zero, a couple of notes a second and a high register, and be one \
         instrument to anyone listening. \
         Then the counts across the project: how many songs use each source \
         kind, how many carry it in their loudest track, the spread of tempos \
         and of registers. Reads the recipe documents only — no bake, no \
         ffmpeg, no network, no cost. It counts and stops: there is no score, \
         no grade and no recommendation here, and nothing about it can fail \
         anything. A project of fewer than two songs has no set to report on \
         and says so in one line."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        project_only_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let project = load(&dir)?;
        let surveyed = synth::survey(&project, &dir).map_err(|error| format!("{error}"))?;
        let lines = scorsese_render::say::survey(&surveyed);
        if lines.is_empty() {
            // The report itself says nothing below two songs, exactly as the
            // CLI prints nothing. A tool call cannot answer with silence,
            // though — a client shows what came back — so the count it
            // declined to report on is what comes back instead.
            return Ok(format!(
                "{} song recipes — a survey reports across a set of them.",
                surveyed.songs.len()
            )
            .into());
        }
        Ok(lines.join("\n").into())
    }
}
