//! The plain values of one placed clip: how fast it plays, how it meets the
//! frame, and where its picture sits.
//!
//! These are the fields a person reaches for in an inspector — the desktop
//! app's (`app/src/inspector/`) and the web editor's (#545) — and this is the
//! one implementation of them, so a window and an assistant write the same
//! document for the same request. Speed is [`Clip::retime`], the reading an
//! editor's 2× button means; position, rotation and scale are
//! [`level::set`] with a [`Level::Flat`], the call `set_volume` makes for a
//! level — one ordinary keyframe held from the clip's first frame. Shadow,
//! glow and blend are [`light`]'s, since each is a small object of its own,
//! and a track matte is [`matte`]'s for the same reason.

mod light;
mod matte;

use schemars::JsonSchema;
use scorsese_core::level::{self, Level};
use scorsese_core::{Clip, ClipId, Fit, KeyframeTrack, Project, PropertyPath, Speed, TrackKind};
use scorsese_render::picture::path::{POSITION_X, POSITION_Y, ROTATION, SCALE_X, SCALE_Y};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Set a clip's speed, fit, position, rotation, scale, shadow, glow, blend or
/// matte.
pub(crate) struct ClipSet;

/// What `clip_set` takes.
///
/// Shadow, glow and matte stay JSON here and are read by [`light`] and
/// [`matte`]: an object merges into what the clip has and `false` removes it,
/// and their refusals name the field inside the object that was wrong.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the clip to change. project_describe and project_read name them.
    clip: Name,
    /// How fast the source plays: 2.0 is twice as fast, 0.5 half. The clip
    /// keeps the footage it shows and its length on the timeline changes to fit
    /// it, rounded to whole frames. Positive.
    speed: Option<f64>,
    /// How the picture meets the frame: `fit` inside it with the rest
    /// transparent, `fill` covering it with the overflow cropped, `native` at
    /// the source's own pixel size. Picture only.
    #[schemars(extend("enum" = ["fit", "fill", "native"]))]
    fit: Option<String>,
    /// How far right of where it naturally sits, as a fraction of the frame's
    /// width: 0.25 is a quarter of the way across, negative is left. Picture
    /// only.
    position_x: Option<f64>,
    /// How far below where it naturally sits, as a fraction of the frame's
    /// height: negative is up. Picture only.
    position_y: Option<f64>,
    /// Turn about its centre, in degrees. Positive is clockwise. Picture only.
    rotation: Option<f64>,
    /// Size as a multiplier of its natural size, the same both ways: 0.5 is
    /// half, 2.0 double. Positive — a flip is its own property. Picture only.
    scale: Option<f64>,
    /// A drop shadow: the clip's own silhouette, softened, offset and drawn
    /// under it. An object sets the fields it names and keeps the rest (the
    /// defaults, on a clip without one: black, 0.01 down and right, softness
    /// 0.02, opacity 0.5); `false` removes it. Lengths are fractions of the
    /// clip's own height, as `blur` is. Picture only.
    #[serde(default)]
    #[schemars(schema_with = "light::shadow")]
    shadow: Option<Value>,
    /// A soft halo of light round whatever the clip draws, drawn under it. An
    /// object sets the fields it names and keeps the rest (the defaults, on a
    /// clip without one: the clip's own colours, radius 0.02, intensity 1);
    /// `false` removes it. On a group clip it lights the whole group. Picture
    /// only.
    #[serde(default)]
    #[schemars(schema_with = "light::glow")]
    glow: Option<Value>,
    /// How the clip lands on what is beneath it, shadow and glow included:
    /// `normal` covers; `add` and `screen` add light, so overlapping glowing
    /// things brighten; `multiply` darkens. Over nothing but the black frame,
    /// `add` and `screen` look exactly like `normal`. Picture only.
    #[schemars(extend("enum" = ["normal", "add", "screen", "multiply"]))]
    blend: Option<String>,
    /// Show this clip only through another clip's picture — a track matte, for
    /// a wipe, an iris or footage through the letters of a title. The matte clip
    /// is used only as a mask and is no longer drawn itself; whatever animates
    /// it (a scale growing, a blur softening its edge) animates the reveal. It
    /// must be on the same timeline (both inside one group, or both outside),
    /// must not be this clip, and must not have a matte of its own. An object
    /// sets the fields it names and keeps the rest; `false` removes the matte,
    /// and the matte clip is drawn again. Picture only.
    #[serde(default)]
    #[schemars(schema_with = "matte::schema")]
    matte: Option<Value>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("clip", "the id of the clip to change")];
}

impl Arguments {
    /// Each held value given, with its argument's name and the property paths
    /// it holds. Scale is both axes: the one scale a person means is the
    /// picture bigger or smaller, in proportion.
    fn held(&self) -> impl Iterator<Item = (&'static str, f64, &'static [&'static str])> {
        [
            ("position_x", self.position_x, &[POSITION_X][..]),
            ("position_y", self.position_y, &[POSITION_Y][..]),
            ("rotation", self.rotation, &[ROTATION][..]),
            ("scale", self.scale, &[SCALE_X, SCALE_Y][..]),
        ]
        .into_iter()
        .filter_map(|(key, value, paths)| value.map(|value| (key, value, paths)))
    }

    /// The first of the light and matte arguments given, if any.
    fn lit(&self) -> Option<&'static str> {
        [
            ("shadow", self.shadow.is_some()),
            ("glow", self.glow.is_some()),
            ("blend", self.blend.is_some()),
            ("matte", self.matte.is_some()),
        ]
        .into_iter()
        .find_map(|(key, given)| given.then_some(key))
    }
}

impl Tool for ClipSet {
    fn name(&self) -> &'static str {
        "clip_set"
    }

    fn description(&self) -> &'static str {
        "Change a placed clip's plain values: its speed, fit, position, \
         rotation, scale, shadow, glow, blend and matte. Position, rotation and \
         scale are single values held for the whole clip; shadow, glow and blend \
         are its light — a drop shadow, a halo, and how it lands on what is \
         beneath it; matte shows it only through another clip's picture, for a \
         wipe or a reveal. These are the fields an inspector shows. **Every argument you \
         leave out is left exactly as it is.** A speed retimes the clip the way \
         an editor's 2× button does: the same footage in less time, so its length \
         changes with it. Position, rotation and scale are written as one \
         ordinary keyframe held from the clip's first frame, exactly what \
         set_volume writes for a level — so a property that was animated is \
         flattened, and the reply names what it replaced. Where the clip starts \
         and how long it runs are clip_move's. Nothing is written unless the \
         whole document still loads."
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
        let id = ClipId::new(arguments.clip.as_str());
        let said = set(&mut project, &id, &arguments)?;
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(format!("`{id}`: {}. Nothing else changed.", said.join("; ")).into())
    }
}

/// Apply every value `arguments` names to clip `id`, on a copy that becomes
/// `project` only if it validates; what was set, in words.
fn set(project: &mut Project, id: &ClipId, arguments: &Arguments) -> Result<Vec<String>, String> {
    let mut proposed = project.clone();
    let (kind, clip) = find(&mut proposed, id)?;
    let picture = kind == TrackKind::Video;
    let mut said = Vec::new();

    if let Some(rate) = arguments.speed {
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
    if let Some(fit) = &arguments.fit {
        let fit = Value::String(fit.clone());
        let fit: Fit = serde_json::from_value(fit.clone())
            .map_err(|_| format!("`fit` is `fit`, `fill` or `native`, not {fit}"))?;
        only_picture(picture, "fit")?;
        find(&mut proposed, id)?.1.fit = fit;
        said.push(format!("fit {}", fit_name(fit)));
    }
    for (key, value, paths) in arguments.held() {
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
    let mut lit = light::apply(find(&mut proposed, id)?.1, arguments)?;
    lit.extend(matte::apply(
        find(&mut proposed, id)?.1,
        arguments.matte.as_ref(),
    )?);
    if let Some(key) = arguments.lit() {
        only_picture(picture, key)?;
    }
    said.extend(lit);
    if said.is_empty() {
        return Err(
            "nothing to set: name at least one of speed, fit, position_x, position_y, \
                    rotation, scale, shadow, glow, blend or matte"
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
