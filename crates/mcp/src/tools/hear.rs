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
//!
//! **And the numbers with it** (#782). This used to stop at the picture and
//! send the reader to a second tool, `audio_level`, for mean, peak, crest,
//! spectral balance and stereo width. Both decoded the same file to answer the
//! same question — *how did this sound file come out?* — and every client that
//! was ever recorded picked this one and never the other (#779). So the numbers
//! now ride in the text above the picture: one summary line by default, the
//! per-section rows behind `sections`, and a field-by-field comparison behind
//! `against`. The default stays lean because a reply is re-read on every model
//! call after it, and the picture is already most of its weight.

use std::path::Path;

use schemars::JsonSchema;
use scorsese_render::audio::{Waveform, measure, waveform};
use scorsese_render::{Tools, frames, say};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::scratch::Scratch;
use crate::tools::{Costs, Part, Reply, Tool};

/// A waveform of a sound file, and its levels.
pub(crate) struct Hear;

/// What `hear` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The sound to look at, as a path relative to the project — e.g.
    /// generated/vo-01.mp3 — or an absolute path to a file outside it. Not an
    /// asset id: this reads a file, and the file need not be in the assets
    /// table. A rendered video works too; its sound is what is read.
    file: String,
    /// Compare with this file, field by field: mean, peak, crest, balance,
    /// width and length, each with how far it moved. Usually the version the
    /// first one was meant to replace — the previous bake, or the render that
    /// sounded right. Relative to the project, or absolute.
    against: Option<String>,
    /// Add one row of levels per section of the file — the same columns as the
    /// summary, for each stretch of it. Off by default: the summary line is
    /// usually the answer, and the rows grow with the file's length.
    #[serde(default)]
    sections: bool,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("file", "the path of a sound file to look at")];
}

impl Tool for Hear {
    fn name(&self) -> &'static str {
        "hear"
    }

    fn description(&self) -> &'static str {
        "See and measure a sound file: its waveform as one picture, and its \
         levels in words. The levels are its length, mean and peak in dBFS, the \
         share of its energy that is low, mid and high, and how much is common \
         to both channels (`corr`: +1.00 is mono in a stereo container; \
         negative means the sides cancel and the mix collapses in mono). The \
         audio counterpart of `look`. Use it on generated narration before trusting it, and after \
         rewriting a score — a duration and a byte hash cannot tell 3.7 seconds \
         of speech from 3.7 seconds of silence, and this can. It answers: is it \
         silent, is it clipped, does it start late or end early, is it the \
         length it should be, where are the pauses, and how loud and how bright \
         it is. Give `against` to compare two files field by field — the form \
         that answers \"did my change land?\", since an absolute level is hard \
         to judge and a difference is not — and `sections` for one row of \
         levels per section. It does NOT answer whether the voice, the language \
         or the pronunciation are right — those need hearing, and nothing here \
         can tell you about them. Works on any audio the project can reach and \
         on a rendered video too, since ffmpeg does the decoding."
    }

    fn costs(&self) -> Costs {
        Costs::Decode
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let file = args::path(dir, &arguments.file, "file")?;
        let other = args::under(dir, arguments.against.as_deref(), "against")?;

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

        let mut text = said(&wave);
        text.push_str(&levels(
            &tools,
            &file,
            other.as_deref(),
            arguments.sections,
        )?);
        Ok(vec![Part::picture(text, &bytes)].into())
    }
}

/// The numbers under the sentence: a summary line, the section rows when they
/// were asked for, and the comparison when there is something to compare with.
///
/// Measured on a second decode rather than folded into the waveform's, because
/// the two are separate measurements in `render` with separate callers (the
/// CLI's `level` and `hear` among them), and one more decode of a file the
/// call has just read is cheap next to the picture it already pays for.
fn levels(
    tools: &Tools,
    file: &Path,
    other: Option<&Path>,
    sections: bool,
) -> Result<String, String> {
    let profile = measure(tools, file).map_err(|error| format!("{error}"))?;
    let mut text = format!("\nlevels  {}", say::summary(&profile));
    if sections {
        for row in say::sections(&profile) {
            text.push_str(&format!("\n  {row}"));
        }
    }
    if let Some(other) = other {
        let previous = measure(tools, other).map_err(|error| format!("{error}"))?;
        text.push_str(&format!("\n\n{}  vs  {}", name(file), name(other)));
        for row in say::comparison(&profile, &previous) {
            text.push_str(&format!("\n  {row}"));
        }
    }
    Ok(text)
}

/// How a file is named in a comparison: its file name, not its whole path. Two
/// files being compared usually differ in one word, and two long paths that
/// agree for sixty characters bury it.
fn name(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
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
