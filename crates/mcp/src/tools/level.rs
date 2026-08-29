//! How a finished sound file came out, and how it differs from another.
//!
//! The tool an assistant reaches for after it has rewritten a score. It cannot
//! hear, so for every other part of this project it can check its own work and
//! for audio the loop currently terminates at a human. Most of that gap is not
//! real — for a `synth_audio` asset the assistant *wrote the document*, and
//! which instruments play and where the sections are is in the recipe already.
//! The gap that is real is everything that only exists **after** the render:
//! level, spectral balance, and whether section C is actually bigger than
//! section A or merely has more notes in it. That is the whole of what this
//! answers.

use std::path::{Path, PathBuf};

use scorsese_render::{Tools, audio::measure, say};
use serde_json::Value;

use crate::tools::Reply;

use super::{Costs, Tool, project_dir, project_property};

/// Measure a finished sound file, optionally against another.
pub(crate) struct Level;

impl Tool for Level {
    fn name(&self) -> &'static str {
        "audio_level"
    }

    fn description(&self) -> &'static str {
        "Say how a finished sound file came out. Mean, peak and crest over its \
         whole length, then the same again for each section of it, plus the \
         share of its energy that is low, mid and high and how much of it is \
         common to both channels (`corr`: +1.00 is mono in a stereo container, \
         and a negative reading means the two sides are cancelling and the mix \
         collapses in mono). Give `against` and the \
         two are compared field by field, which is the form that actually \
         answers \"did my change land?\" — an absolute level is hard to judge \
         and a difference is not. A signal, never a gate: there is no correct \
         loudness, so nothing here can fail. Works on a bake or on a rendered \
         video, since ffmpeg does the decoding."
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
                    "description": "The file to measure, relative to the project \
                                    directory or absolute. A bake under generated/, \
                                    a rendered video, or any media with sound in it."
                },
                "against": {
                    "type": "string",
                    "description": "Compare with this file, the same way. Usually the \
                                    version the first one was meant to replace — the \
                                    previous bake, or the render that sounded right."
                }
            },
            "required": ["project", "file"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let file = under(&dir, arguments, "file")?
            .ok_or_else(|| "`file` is required: the sound file to measure".to_owned())?;
        // Discovered per call rather than held, for the reason the render tool
        // gives: a server that found ffmpeg at startup would keep insisting it
        // was there after someone uninstalled it.
        let tools = Tools::discover().map_err(|error| format!("{error}"))?;

        let profile = measure(&tools, &file).map_err(|error| format!("{error}"))?;
        let mut said = say::headline(&name(&file), &profile);
        for row in say::sections(&profile) {
            said.push_str(&format!("\n  {row}"));
        }

        let Some(other) = under(&dir, arguments, "against")? else {
            return Ok(said.into());
        };
        let previous = measure(&tools, &other).map_err(|error| format!("{error}"))?;
        said.push_str(&format!("\n\n{}  vs  {}", name(&file), name(&other)));
        for row in say::comparison(&profile, &previous) {
            said.push_str(&format!("\n  {row}"));
        }
        Ok(said.into())
    }
}

/// One path argument, resolved against the project directory unless it is
/// already absolute.
///
/// Relative-to-the-project is the rule everything else in a project obeys, and
/// an absolute path is still allowed because a delivered render is as likely to
/// sit outside the project as in it. Nothing is written here, so the usual
/// reason to refuse a path that escapes the root does not apply.
fn under(dir: &Path, arguments: &Value, field: &str) -> Result<Option<PathBuf>, String> {
    let Some(given) = arguments.get(field).and_then(Value::as_str) else {
        return Ok(None);
    };
    if given.trim().is_empty() {
        return Err(format!("`{field}` is empty — give a path or leave it out"));
    }
    let path = PathBuf::from(given);
    Ok(Some(if path.is_absolute() {
        path
    } else {
        dir.join(path)
    }))
}

/// How a file is named in the reply: its file name, not its whole path. Two
/// files being compared usually differ in one word, and two long paths that
/// agree for sixty characters bury it.
fn name(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
