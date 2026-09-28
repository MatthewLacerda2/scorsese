//! The plain values of one placed clip: how fast it plays, how it meets the
//! frame, and where its picture sits.
//!
//! These are the fields a person reaches for in an inspector — the desktop
//! app's (`app/src/inspector/`) and the web editor's (#545) — and this is the
//! one implementation of them, so a window and an assistant write the same
//! document for the same request. Speed is [`Clip::retime`], the reading an
//! editor's 2× button means; position, rotation and scale are
//! [`level::set`] with a [`Level::Flat`], the call `set_volume` makes for a
//! level — one ordinary keyframe held from the clip's first frame.

use scorsese_core::level::{self, Level};
use scorsese_core::{Clip, ClipId, Fit, KeyframeTrack, Project, PropertyPath, Speed, TrackKind};
use scorsese_render::picture::path::{POSITION_X, POSITION_Y, ROTATION, SCALE_X, SCALE_Y};
use serde_json::Value;

use super::named;
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Set a clip's speed, fit, position, rotation or scale.
pub(crate) struct ClipSet;

/// Each value argument, the property paths it holds, and what it is called in
/// a reply. Scale is both axes: the one scale a person means is the picture
/// bigger or smaller, in proportion.
const HELD: [(&str, &[&str]); 4] = [
    ("position_x", &[POSITION_X]),
    ("position_y", &[POSITION_Y]),
    ("rotation", &[ROTATION]),
    ("scale", &[SCALE_X, SCALE_Y]),
];

impl Tool for ClipSet {
    fn name(&self) -> &'static str {
        "clip_set"
    }

    fn description(&self) -> &'static str {
        "Change a placed clip's plain values: its speed, its fit, and its \
         position, rotation and scale as single values held for the whole clip. \
         These are the fields an inspector shows. **Every argument you \
         leave out is left exactly as it is.** A speed retimes the clip the way \
         an editor's 2× button does: the same footage in less time, so its length \
         changes with it. Position, rotation and scale are written as one \
         ordinary keyframe held from the clip's first frame, exactly what \
         set_volume writes for a level — so a property that was animated is \
         flattened, and the reply names what it replaced. Where the clip starts \
         and how long it runs are trim_clip's. Nothing is written unless the \
         whole document still loads."
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
                    "description": "Id of the clip to change. project_describe and \
                                    project_read name them."
                },
                "speed": {
                    "type": "number",
                    "description": "How fast the source plays: 2.0 is twice as fast, \
                                    0.5 half. The clip keeps the footage it shows and \
                                    its length on the timeline changes to fit it, \
                                    rounded to whole frames. Positive."
                },
                "fit": {
                    "type": "string",
                    "enum": ["fit", "fill", "native"],
                    "description": "How the picture meets the frame: `fit` inside it \
                                    with the rest transparent, `fill` covering it with \
                                    the overflow cropped, `native` at the source's own \
                                    pixel size. Picture only."
                },
                "position_x": {
                    "type": "number",
                    "description": "How far right of where it naturally sits, as a \
                                    fraction of the frame's width: 0.25 is a quarter \
                                    of the way across, negative is left. Picture only."
                },
                "position_y": {
                    "type": "number",
                    "description": "How far below where it naturally sits, as a \
                                    fraction of the frame's height: negative is up. \
                                    Picture only."
                },
                "rotation": {
                    "type": "number",
                    "description": "Turn about its centre, in degrees. Positive is \
                                    clockwise. Picture only."
                },
                "scale": {
                    "type": "number",
                    "description": "Size as a multiplier of its natural size, the same \
                                    both ways: 0.5 is half, 2.0 double. Positive — a \
                                    flip is its own property. Picture only."
                }
            },
            "required": ["project", "clip"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let id = ClipId::new(named(arguments, "clip", "the id of the clip to change")?);
        let said = set(&mut project, &id, arguments)?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!("`{id}`: {}. Nothing else changed.", said.join("; ")).into())
    }
}

/// Apply every value `arguments` names to clip `id`, on a copy that becomes
/// `project` only if it validates; what was set, in words.
fn set(project: &mut Project, id: &ClipId, arguments: &Value) -> Result<Vec<String>, String> {
    let mut proposed = project.clone();
    let (kind, clip) = find(&mut proposed, id)?;
    let picture = kind == TrackKind::Video;
    let mut said = Vec::new();

    if let Some(rate) = number(arguments, "speed")? {
        let speed = Speed::new(rate);
        if !speed.is_usable() {
            return Err(format!(
                "a speed of {rate} is not one a clip can play at — it has to be positive"
            ));
        }
        clip.retime(speed);
        let fps = proposed.timeline_fps;
        let clip = find(&mut proposed, id)?.1;
        said.push(format!(
            "speed {rate}, so it now runs {:.2}s",
            fps.seconds(clip.duration)
        ));
    }
    if let Some(fit) = arguments.get("fit").filter(|fit| !fit.is_null()) {
        let fit: Fit = serde_json::from_value(fit.clone())
            .map_err(|_| format!("`fit` is `fit`, `fill` or `native`, not {fit}"))?;
        only_picture(picture, "fit")?;
        find(&mut proposed, id)?.1.fit = fit;
        said.push(format!("fit {}", fit_name(fit)));
    }
    for (key, paths) in HELD {
        let Some(value) = number(arguments, key)? else {
            continue;
        };
        only_picture(picture, key)?;
        if key == "scale" && value <= 0.0 {
            return Err(format!(
                "a scale of {value} is not a size — it has to be above zero"
            ));
        }
        let mut replaced = Vec::new();
        for path in paths {
            let levelled = level::set(
                &mut proposed,
                id,
                &PropertyPath::new(*path),
                Level::Flat(value),
            )
            .map_err(|error| format!("{key}: {error} — nothing was written"))?;
            replaced.extend(
                levelled
                    .replaced
                    .iter()
                    .filter(|track| !single(track))
                    .cloned(),
            );
        }
        said.push(format!(
            "{} {value}{}",
            key.replace('_', " "),
            flattened(&replaced)
        ));
    }
    if said.is_empty() {
        return Err(
            "nothing to set: name at least one of speed, fit, position_x, position_y, \
                    rotation or scale"
                .to_owned(),
        );
    }
    proposed.validate().map_err(|errors| {
        let problems: Vec<String> = errors.into_vec().iter().map(ToString::to_string).collect();
        format!("{} — nothing was written", problems.join("; "))
    })?;
    *project = proposed;
    Ok(said)
}

/// The kind of track clip `id` is on, and the clip.
fn find<'a>(project: &'a mut Project, id: &ClipId) -> Result<(TrackKind, &'a mut Clip), String> {
    project
        .tracks
        .iter_mut()
        .find_map(|track| {
            let kind = track.kind;
            track
                .clips
                .iter_mut()
                .find(|clip| &clip.id == id)
                .map(|clip| (kind, clip))
        })
        .ok_or_else(|| format!("there is no clip `{id}` in this project"))
}

/// A number argument, when it is given; refused when it is not a finite one.
fn number(arguments: &Value, key: &str) -> Result<Option<f64>, String> {
    match arguments.get(key).filter(|value| !value.is_null()) {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(Some)
            .ok_or_else(|| format!("`{key}` has to be a number")),
    }
}

/// Refuse a picture value on a sound: an audio clip has no raster, and a value
/// set there would look applied while changing nothing.
fn only_picture(picture: bool, key: &str) -> Result<(), String> {
    if picture {
        Ok(())
    } else {
        Err(format!(
            "`{key}` is about a picture, and this clip is on an audio track"
        ))
    }
}

/// True for a track that held one value — what this tool writes, and so no
/// loss worth mentioning when it is replaced.
fn single(track: &KeyframeTrack) -> bool {
    track.keyframes.len() == 1
}

/// What replacing `tracks` flattened, said as the edit it was.
fn flattened(tracks: &[KeyframeTrack]) -> String {
    if tracks.is_empty() {
        return String::new();
    }
    let each: Vec<String> = tracks
        .iter()
        .map(|track| {
            let points = track.keyframes.len();
            match &track.by {
                Some(tool) => format!("{points} points `{tool}` wrote"),
                None => format!("{points} points written by hand"),
            }
        })
        .collect();
    format!(" (replacing an animation: {})", each.join(", "))
}

/// A fit as the document spells it.
fn fit_name(fit: Fit) -> &'static str {
    match fit {
        Fit::Fit => "fit",
        Fit::Fill => "fill",
        Fit::Native => "native",
    }
}
