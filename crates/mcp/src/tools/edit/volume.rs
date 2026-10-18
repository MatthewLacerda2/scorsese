//! Setting how loud one clip plays.

use schemars::JsonSchema;
use scorsese_core::level::{self, Level, Levelled};
use scorsese_core::{ClipId, Frames, Project, PropertyPath};
use scorsese_render::audio::path::VOLUME;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Set a clip's volume.
pub(crate) struct SetVolume;

/// What `set_volume` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the clip to set the volume of. Any clip that makes a sound, which
    /// includes a clip on a video track whose file has audio on it.
    clip: Name,
    /// How loud the clip plays, as a multiplier on its own level: 1.0 is the
    /// source as recorded, 0.5 is half, and above 1.0 is gain. Muting a clip is
    /// a level of 0.0. With `from_level` this is where the fade arrives, and
    /// what the clip plays at from there on.
    level: f64,
    /// Where the volume starts, when it is to arrive as a fade rather than
    /// simply being held — `from_level` 0.0 with `level` 1.0 is a fade in, and
    /// the other way round is a fade out. Needs `seconds`. Omit both for a flat
    /// level over the whole clip.
    from_level: Option<f64>,
    /// How long the fade takes. Rounded to whole frames on the project's grid
    /// and never to zero — a fade of no length is a flat level. Only meaningful
    /// with `from_level`.
    seconds: Option<f64>,
    /// When the fade starts, in seconds from the start of the clip — not from
    /// the start of the timeline. Default 0.0, the head of the clip. Before it
    /// the clip plays at `from_level`. Only meaningful with `from_level`.
    at_seconds: Option<f64>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[
        ("clip", "a clip id"),
        ("level", "1.0 as recorded, 0.0 silent"),
    ];
}

impl Tool for SetVolume {
    fn name(&self) -> &'static str {
        "set_volume"
    }

    fn description(&self) -> &'static str {
        "Set how loud one clip plays — a level, a mute, or a fade between two \
         points — by writing the ordinary volume keyframes you would place by \
         hand, which stay editable afterwards. It replaces the clip's own \
         hand-written volume keyframes, and the reply names what went; a dip \
         `duck_music` wrote stays, and multiplies with this level, so a \
         ducked clip stays ducked. A fade must finish inside the clip, or \
         nothing is changed at all."
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
        let clip = ClipId::new(arguments.clip.as_str());

        if let Some(group) = project.group_holding(&clip) {
            return Err(format!(
                "`{clip}` is inside group `{group}`, and a group's members are not heard — \
                 a group is picture only, so a volume there would change nothing"
            ));
        }
        let asked = asked_for(&arguments, project.timeline_fps.as_f64())?;
        let report = level::set(&mut project, &clip, &PropertyPath::new(VOLUME), asked)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        save(&project, dir)?;

        let kept = kept(&project, &clip);
        Ok(format!(
            "{}{}{}",
            wrote(&clip, asked, kept.is_empty()),
            beside(&kept, matches!(asked, Level::Flat(level) if level == 0.0)),
            replaced(&report)
        )
        .into())
    }
}

/// The level the arguments describe, or why they describe none.
fn asked_for(arguments: &Arguments, fps: f64) -> Result<Level, String> {
    let to = audible(Some(arguments.level), "level")?.unwrap_or_default();
    let from = audible(arguments.from_level, "from_level")?;
    match (from, arguments.seconds) {
        (Some(from), Some(seconds)) => Ok(Level::Ramp {
            from,
            to,
            at: Frames::span(arguments.at_seconds.unwrap_or(0.0), fps),
            over: Frames::span(seconds, fps),
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
        (None, None) if arguments.at_seconds.is_some() => Err(
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
fn audible(given: Option<f64>, key: &str) -> Result<Option<f64>, String> {
    match given {
        Some(value) if value < 0.0 => Err(format!(
            "`{key}` of {value} is below silence — volume is a multiplier, so mute a clip with 0.0"
        )),
        found => Ok(found),
    }
}

/// The tools whose volume tracks stayed on the clip beside the new level —
/// a dip `duck_music` signed, which multiplies with it (#1099).
fn kept(project: &Project, clip: &ClipId) -> Vec<String> {
    project.clip(clip).map_or_else(Vec::new, |found| {
        found
            .keyframes
            .iter()
            .filter(|track| track.property.as_str() == VOLUME)
            .filter_map(|track| track.by.clone())
            .collect()
    })
}

/// What the tool just wrote, in the terms it was asked for. `alone` is false
/// when a tool's volume still multiplies with it, so the level is the clip's
/// own rather than what it plays at.
fn wrote(clip: &ClipId, level: Level, alone: bool) -> String {
    match level {
        Level::Flat(value) => flat(clip, value, alone),
        Level::Ramp { from, to, at, over } if alone => format!(
            "`{clip}` fades {} → {} over {over} frames from frame {at} of the clip, and plays \
             at {} after that — two ordinary volume keyframes, editable and deletable.",
            said(from),
            said(to),
            said(to)
        ),
        Level::Ramp { from, to, at, over } => format!(
            "`{clip}`'s own level fades {} → {} over {over} frames from frame {at} of the \
             clip, and holds at {} after that — two ordinary volume keyframes, editable and \
             deletable.",
            said(from),
            said(to),
            said(to)
        ),
    }
}

/// A flat level, said as the edit it is: a mute is worth naming, because
/// "plays at 0.0" is a sentence nobody would write about a silent clip, and
/// stays true whatever multiplies with it.
fn flat(clip: &ClipId, value: f64, alone: bool) -> String {
    if value == 0.0 {
        format!("`{clip}` is muted — volume 0.0 for the whole clip, as one ordinary keyframe.")
    } else if alone {
        format!(
            "`{clip}` plays at {} for the whole clip — one ordinary volume keyframe, editable \
             and deletable.",
            said(value)
        )
    } else {
        format!(
            "`{clip}`'s own level is {} for the whole clip — one ordinary volume keyframe, \
             editable and deletable.",
            said(value)
        )
    }
}

/// The tools' volume that stayed, said so the level above is not read as what
/// the clip plays at: the two multiply, so a ducked clip still dips. Under a
/// mute it is only still there, ready for when the clip is turned back up.
fn beside(kept: &[String], muted: bool) -> String {
    if kept.is_empty() {
        return String::new();
    }
    let each: Vec<String> = kept.iter().map(|tool| format!("`{tool}`")).collect();
    let tools = each.join(" and ");
    if muted {
        format!(" The volume track signed {tools} stays on the clip, under the mute.")
    } else {
        format!(
            " The volume track signed {tools} stays on the clip and multiplies with this \
             level, so the clip still dips where it did, now from this level."
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

/// What the new track displaced: only ever the clip's own hand-written volume,
/// since a dip `duck_music` signed stays and multiplies with the new level.
fn replaced(report: &Levelled) -> String {
    if report.replaced.is_empty() {
        return String::new();
    }
    let each: Vec<String> = report
        .replaced
        .iter()
        .map(|track| format!("{} point(s)", track.keyframes.len()))
        .collect();
    format!(
        " It replaced the hand-written volume keyframes that were on the clip ({}).",
        each.join(", ")
    )
}
