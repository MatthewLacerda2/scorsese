//! Animating one property of a placed clip: the generic keyframe tool.
//!
//! Every other tool that writes keyframes is for one job — `set_volume` a
//! level, `dissolve` a crossover, `clip_follow` a trip along an arrow — and
//! `clip_set` holds a single value. This is the mechanism itself (#646): any
//! property the build publishes as animatable, any easing the format has, in
//! seconds rather than frames, without rewriting the whole document.
//!
//! **It takes the property's lane over**, as `set_volume` does and for the same
//! reason: a renderer resolves a path by finding *the* track for it, so a
//! second track on one property is one that silently never plays. Every other
//! property's track is left exactly as it was.
//!
//! **What it writes is unsigned.** A signature (`by`) is how a generator finds
//! its own earlier output to redo it, and the points here are the caller's
//! own decisions, not a pattern recomputed from the edit — so nothing may
//! replace them on a re-run, which is what an unsigned track means.

mod points;

use schemars::JsonSchema;
use scorsese_core::{Clip, ClipId, KeyframeTrack, Project, PropertyPath};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};
use points::{Asked, Point, points, properties, spelled};

/// Write, replace or remove one keyframe track on a clip.
pub(crate) struct ClipAnimate;

/// What `clip_animate` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Id of the clip to animate — one on the timeline, or a member of a group.
    /// project_describe and project_read name them.
    clip: Name,
    /// What to animate, as a property path: `opacity`, `transform.position.x`,
    /// `transform.rotation`, `shape.trim_end`, `reveal`, `number`,
    /// `glow.intensity`, `volume` … — the animatable table in
    /// docs/project-format.md is the whole list, with what each number means.
    /// `transform.scale`, `transform.position` and `transform.flip` write both
    /// axes at once. An unknown path is refused with the closest one.
    property: Name,
    /// The points the property passes through, replacing any it had. Before the
    /// first it holds the first value, after the last the last. Empty removes
    /// the animation, so the clip goes back to its default.
    keyframes: Vec<Asked>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[
        ("clip", "the id of the clip to animate"),
        ("property", "a property path, such as `opacity`"),
        (
            "keyframes",
            "a list of { at_seconds, value, easing } — or [] to stop animating the property",
        ),
        ("at_seconds", "seconds from the clip's start"),
        ("value", "what the property reads at that moment"),
    ];
}

impl Tool for ClipAnimate {
    fn name(&self) -> &'static str {
        "clip_animate"
    }

    fn description(&self) -> &'static str {
        "Animate one property of a placed clip by writing its keyframes, in \
         seconds from the clip's start and with easing by name. Opacity, position, \
         scale, a shape drawing itself on, a caption revealing, a glow pulsing: \
         any path in the animatable table. \
         It replaces that one property's keyframes on the clip, whoever wrote them, \
         and leaves every other property's alone; `keyframes: []` removes the \
         property's animation. The table is in docs/project-format.md, and a pair \
         stem such as `transform.scale` writes both its `.x` and `.y` with the same \
         keyframes. clip_set's position, \
         rotation and scale flatten what this wrote, and this replaces what they \
         held. Works on a clip inside a group too. Nothing is written unless the \
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
        let asked = arguments.property.as_str();
        let paths = properties(asked)?;
        let points = points(&arguments.keyframes, project.timeline_fps)?;
        let replaced = animate(&mut project, &id, &paths, &points)?;
        if points.is_empty() && replaced.is_empty() {
            return Ok(format!("`{id}` was not animating `{asked}`; nothing changed.").into());
        }
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        let fps = project.timeline_fps;
        let wrote = if points.is_empty() {
            format!("{} no longer animated", named_paths(&paths))
        } else {
            let each: Vec<String> = points
                .iter()
                .map(|point| said(point, fps.seconds(point.frame)))
                .collect();
            format!(
                "{} now animated through {} keyframe(s): {} — an ordinary, unsigned track \
                 per property, editable by calling this again",
                named_paths(&paths),
                points.len(),
                each.join(", ")
            )
        };
        Ok(format!("`{id}`: {wrote}.{} Nothing else changed.", gone(&replaced)).into())
    }
}

/// Put one track per path on clip `id`, on a copy that becomes `project` only
/// if it validates; the tracks it displaced.
fn animate(
    project: &mut Project,
    id: &ClipId,
    paths: &[PropertyPath],
    points: &[Point],
) -> Result<Vec<KeyframeTrack>, String> {
    let mut proposed = project.clone();
    let clip = clip_mut(&mut proposed, id)
        .ok_or_else(|| format!("there is no clip `{id}` in this project"))?;
    if let Some(late) = points.iter().find(|point| point.frame > clip.duration) {
        return Err(format!(
            "a keyframe at {}s is frame {} of `{id}`, which is only {} frames long — \
             nothing was written",
            late.seconds,
            late.frame.get(),
            clip.duration.get()
        ));
    }
    let (replaced, kept): (Vec<_>, Vec<_>) = clip
        .keyframes
        .drain(..)
        .partition(|track| paths.contains(&track.property));
    clip.keyframes = kept;
    if !points.is_empty() {
        let keyframes: Vec<_> = points.iter().map(|point| point.keyframe()).collect();
        for path in paths {
            clip.keyframes
                .push(KeyframeTrack::new(path.clone(), keyframes.clone()));
        }
    }
    proposed.validate().map_err(|errors| {
        let problems: Vec<String> = errors.into_vec().iter().map(ToString::to_string).collect();
        format!("{} — nothing was written", problems.join("; "))
    })?;
    *project = proposed;
    Ok(replaced)
}

/// The clip called `id`, on the timeline or among a group's members — clip ids
/// are one namespace for the whole document, so there is only ever one.
fn clip_mut<'a>(project: &'a mut Project, id: &ClipId) -> Option<&'a mut Clip> {
    let members = project
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut())
        .flat_map(|group| group.tracks.iter_mut());
    project
        .tracks
        .iter_mut()
        .chain(members)
        .flat_map(|track| track.clips.iter_mut())
        .find(|clip| &clip.id == id)
}

/// The paths written, as a reply names them.
fn named_paths(paths: &[PropertyPath]) -> String {
    let each: Vec<String> = paths.iter().map(|path| format!("`{path}`")).collect();
    let verb = if paths.len() == 1 { "is" } else { "are" };
    format!("{} {verb}", each.join(" and "))
}

/// One point, in both units and with its easing when it has one worth naming.
fn said(point: &Point, on_grid: f64) -> String {
    let easing = match spelled(point.easing).as_str() {
        "linear" => String::new(),
        name => format!(" {name}"),
    };
    format!(
        "{} at {on_grid:.2}s (frame {}){easing}",
        point.value,
        point.frame.get()
    )
}

/// What the new tracks displaced, and who had written it.
fn gone(replaced: &[KeyframeTrack]) -> String {
    if replaced.is_empty() {
        return String::new();
    }
    let each: Vec<String> = replaced
        .iter()
        .map(|track| {
            let points = track.keyframes.len();
            match &track.by {
                Some(tool) => format!("`{}` ({points} point(s) `{tool}` wrote)", track.property),
                None => format!("`{}` ({points} point(s) written by hand)", track.property),
            }
        })
        .collect();
    format!(" It replaced what was there: {}.", each.join(", "))
}
