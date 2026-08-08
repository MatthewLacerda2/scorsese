//! Sound, as a picture the client can actually read.
//!
//! The audio counterpart of [`look`](super::look). That one closed the loop on
//! generated video — Veo returns a clip that may or may not be what the brief
//! asked for, and without a picture nobody can tell. This closes the same loop
//! on generated narration, which until now was the one output in the project
//! **nobody had ever checked**.
//!
//! Every audible thing in scorsese used to be either imported, in which case a
//! person heard it before importing it, or synthesised, in which case it is
//! deterministic and reproducible from the document. Generated narration is
//! neither: it costs money, it cannot be reproduced from the project, and the
//! only verification available was file size, byte hash and a probed duration —
//! all three of which a file of pure silence passes.
//!
//! **A picture, not the audio itself.** MCP can carry an audio content block
//! and that would be strictly more capable — it is the only version that
//! catches the wrong voice or the wrong language — but it depends on the client
//! handling audio, and this server may not assume which client is on the other
//! end. The waveform needs nothing but ffmpeg and works everywhere. The other
//! route is not rejected; it is a different issue.

use scorsese_render::audio::{Waveform, waveform};
use scorsese_render::{Tools, frames};
use serde_json::Value;

use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool, project_dir, project_property};

/// A waveform of a sound file.
pub(crate) struct Hear;

impl Tool for Hear {
    fn name(&self) -> &'static str {
        "hear"
    }

    fn description(&self) -> &'static str {
        "See what a sound file looks like: its waveform, drawn as one picture, \
         with the level and the length written on it. The audio counterpart of \
         `look`. Use it on generated narration before trusting it, and after \
         rewriting a score — a duration and a byte hash cannot tell 3.7 seconds \
         of speech from 3.7 seconds of silence, and this can. It answers: is it \
         silent, is it clipped, does it start late or end early, is it the \
         length it should be, and where are the pauses. It does NOT answer \
         whether the voice, the language or the pronunciation are right — those \
         need hearing, and nothing here can tell you about them. Works on any \
         audio the project can reach and on a rendered video too, since ffmpeg \
         does the decoding. For numbers rather than a shape — mean, peak, crest, \
         spectral balance, section by section — use `audio_level` instead; the \
         two answer different questions and neither replaces the other."
    }

    fn costs(&self) -> Costs {
        Costs::Decode
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "file": {
                    "type": "string",
                    "description": "The sound to look at, as a path relative to the \
                                    project — e.g. generated/vo-01.mp3 — or an absolute \
                                    path to a file outside it. Not an asset id: this reads \
                                    a file, and the file need not be in the assets table. \
                                    A rendered video works too; its sound is what is read."
                }
            },
            "required": ["project", "file"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let named = arguments
            .get("file")
            .and_then(Value::as_str)
            .ok_or("`file` is required: the path of a sound file to look at")?;
        let file = dir.join(named);

        // Discovered per call rather than held, as the other ffmpeg tools do: a
        // server that found ffmpeg at startup would keep insisting it was there
        // after somebody uninstalled it.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;
        let wave = waveform(&tools, &file).map_err(|error| format!("{error}"))?;

        // Through a file, because PNG encoding is ffmpeg's and ffmpeg writes
        // files. Nothing is left behind: this is a thing to look at, not an
        // artifact of the project, and `scorsese hear` is how one gets kept.
        let png = Scratch::at(None);
        frames::write_png(&tools, &png.path, &wave.image)
            .map_err(|error| format!("writing the waveform: {error}"))?;
        let bytes = std::fs::read(&png.path)
            .map_err(|error| format!("reading {} back: {error}", png.path.display()))?;

        Ok(vec![Part::picture(said(&wave), &bytes)].into())
    }
}

/// What the reply says in words.
///
/// Every finding the picture shows, said as well as drawn — the rule the other
/// picture tools already follow, and here it carries more weight than usual.
/// "Silent throughout" is the whole answer to the question this tool exists
/// for, and a client that cannot see the image must not have to infer it.
pub(crate) fn said(wave: &Waveform) -> String {
    let mut text = format!("{} — {:.2}s", wave.file.display(), wave.seconds);
    match wave.findings.peak_dbfs() {
        // Returned early rather than followed by the rest: a silent file is
        // silent all the way through, so "starts 3.00s in" would be the same
        // fact worded as though something were coming.
        None => {
            text.push_str(", SILENT throughout — nothing in this file rose above the noise floor");
            return text;
        }
        Some(db) => text.push_str(&format!(", peak {db:.1} dBFS")),
    }
    if wave.findings.clipped > 0 {
        text.push_str(&format!(
            ", CLIPPED in {} of the picture's columns",
            wave.findings.clipped
        ));
    }
    if wave.findings.lead_in > 0.05 {
        text.push_str(&format!(", starts {:.2}s in", wave.findings.lead_in));
    }
    if wave.findings.lead_out > 0.05 {
        text.push_str(&format!(", ends {:.2}s early", wave.findings.lead_out));
    }
    text.push_str(
        " — the picture shows where the pauses are. It cannot tell you \
         whether the voice, the language or the pronunciation are right.",
    );
    text
}
