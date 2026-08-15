//! Setting how loud one clip plays.

use scorsese_core::level::{self, Level, Levelled};
use scorsese_core::{ClipId, PropertyPath};
use scorsese_render::audio::path::VOLUME;
use serde_json::Value;

use super::{clip_id, frames, number};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Set a clip's volume.
pub(crate) struct SetVolume;

impl Tool for SetVolume {
    fn name(&self) -> &'static str {
        "set_volume"
    }

    fn description(&self) -> &'static str {
        "Set how loud one clip plays — a level, a mute, or a fade between two \
         points — by writing the ordinary volume keyframes you would place by \
         hand, which stay editable afterwards. One clip animates volume from \
         one track, so this takes that lane over: keyframes already on the \
         clip's volume are replaced, including a dip `duck_music` wrote, and \
         the reply names what went. A fade must finish inside the clip, or \
         nothing is changed at all."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "clip": {
                    "type": "string",
                    "description": "Id of the clip to set the volume of. Any clip \
                                    that makes a sound, which includes a clip on a \
                                    video track whose file has audio on it."
                },
                "level": {
                    "type": "number",
                    "description": "How loud the clip plays, as a multiplier on its \
                                    own level: 1.0 is the source as recorded, 0.5 is \
                                    half, and above 1.0 is gain. Muting a clip is a \
                                    level of 0.0. With `from_level` this is where the \
                                    fade arrives, and what the clip plays at from \
                                    there on."
                },
                "from_level": {
                    "type": "number",
                    "description": "Where the volume starts, when it is to arrive as a \
                                    fade rather than simply being held — `from_level` \
                                    0.0 with `level` 1.0 is a fade in, and the other \
                                    way round is a fade out. Needs `seconds`. Omit \
                                    both for a flat level over the whole clip."
                },
                "seconds": {
                    "type": "number",
                    "description": "How long the fade takes. Rounded to whole frames on \
                                    the project's grid and never to zero — a fade of no \
                                    length is a flat level. Only meaningful with \
                                    `from_level`."
                },
                "at_seconds": {
                    "type": "number",
                    "description": "When the fade starts, in seconds from the start of \
                                    the clip — not from the start of the timeline. \
                                    Default 0.0, the head of the clip. Before it the \
                                    clip plays at `from_level`. Only meaningful with \
                                    `from_level`."
                }
            },
            "required": ["project", "clip", "level"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let clip = ClipId::new(clip_id(arguments, "clip")?);

        let asked = asked_for(arguments, project.timeline_fps.as_f64())?;
        let report = level::set(&mut project, &clip, &PropertyPath::new(VOLUME), asked)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        Ok(format!("{}{}", wrote(&clip, asked), replaced(&report)).into())
    }
}

/// The level the arguments describe, or why they describe none.
fn asked_for(arguments: &Value, fps: f64) -> Result<Level, String> {
    let to = audible(arguments, "level")?
        .ok_or_else(|| "`level` is required: 1.0 as recorded, 0.0 silent".to_owned())?;
    let from = audible(arguments, "from_level")?;
    let seconds = arguments.get("seconds").and_then(Value::as_f64);
    match (from, seconds) {
        (Some(from), Some(seconds)) => Ok(Level::Ramp {
            from,
            to,
            at: frames(number(arguments, "at_seconds", 0.0), fps),
            over: frames(seconds, fps),
        }),
        (Some(_), None) => {
            Err("`from_level` needs `seconds` — a fade with no length is a flat level".to_owned())
        }
        (None, Some(_)) => Err(
            "`seconds` needs a `from_level` to fade from — say where the \
                                volume starts, or omit both for a flat level"
                .to_owned(),
        ),
        // Refused rather than ignored: an `at_seconds` on its own reads as
        // "change the volume at this moment", which is a fade missing its
        // other half, and doing nothing about it would look like it worked.
        (None, None) if arguments.get("at_seconds").is_some() => Err(
            "`at_seconds` says when a fade starts, so it needs `from_level` and `seconds` too"
                .to_owned(),
        ),
        (None, None) => Ok(Level::Flat(to)),
    }
}

/// A volume argument, refused when it is below silence.
///
/// A negative multiplier inverts the phase rather than making anything quieter,
/// which is never what dragging a volume line down means — the mixer clamps it
/// away, and a tool that accepted one would be promising an edit that does not
/// happen.
fn audible(arguments: &Value, key: &str) -> Result<Option<f64>, String> {
    match arguments.get(key).and_then(Value::as_f64) {
        Some(value) if value < 0.0 => Err(format!(
            "`{key}` of {value} is below silence — volume is a multiplier, so mute a clip with 0.0"
        )),
        found => Ok(found),
    }
}

/// What the tool just wrote, in the terms it was asked for.
fn wrote(clip: &ClipId, level: Level) -> String {
    match level {
        Level::Flat(value) => flat(clip, value),
        Level::Ramp { from, to, at, over } => format!(
            "`{clip}` fades {} → {} over {over} frames from frame {at} of the clip, and plays \
             at {} after that — two ordinary volume keyframes, editable and deletable.",
            said(from),
            said(to),
            said(to)
        ),
    }
}

/// A flat level, said as the edit it is: a mute is worth naming, because
/// "plays at 0.0" is a sentence nobody would write about a silent clip.
fn flat(clip: &ClipId, value: f64) -> String {
    if value == 0.0 {
        format!("`{clip}` is muted — volume 0.0 for the whole clip, as one ordinary keyframe.")
    } else {
        format!(
            "`{clip}` plays at {} for the whole clip — one ordinary volume keyframe, editable \
             and deletable.",
            said(value)
        )
    }
}

/// A level as a person writes one: `1.0` rather than the `1` a bare float
/// prints as, so the number in the reply is the number to put back in the call.
fn said(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// What the new track displaced, and what to do about it.
fn replaced(report: &Levelled) -> String {
    if report.replaced.is_empty() {
        return String::new();
    }
    let each: Vec<String> = report
        .replaced
        .iter()
        .map(|track| {
            let points = track.keyframes.len();
            match &track.by {
                Some(tool) => format!("one `{tool}` wrote, of {points} point(s)"),
                None => format!("one written by hand, of {points} point(s)"),
            }
        })
        .collect();
    let ducked = report
        .replaced
        .iter()
        .any(|track| track.is_generated_by(scorsese_core::dip::TOOL));
    let again = if ducked {
        " Run `duck_music` again to put the dip back."
    } else {
        ""
    };
    format!(
        " It replaced the volume keyframes that were on the clip — {}.{again}",
        each.join(", ")
    )
}
