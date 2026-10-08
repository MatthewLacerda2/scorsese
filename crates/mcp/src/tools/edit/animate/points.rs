//! Reading a `clip_animate` request: which properties it names, and the
//! keyframes it asks for, on the project's frame grid.

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use scorsese_core::{Easing, Fps, Frames, Keyframe, PropertyPath};
use scorsese_render::ANIMATABLE;
use serde::Deserialize;
use serde_json::Value;

use super::super::seconds;

/// The property paths `asked` names, refused with the closest match when this
/// build animates none of them.
///
/// A path is either one the published vocabulary lists, or the stem of a pair
/// it lists as `.x` and `.y` — `transform.scale` for both axes, which is what
/// "make it pop" means nearly every time. The pair is found in the vocabulary,
/// never named here, so this knows no more about any property than the table
/// in `docs/project-format.md` does.
pub(super) fn properties(asked: &str) -> Result<Vec<PropertyPath>, String> {
    let path = PropertyPath::new(asked);
    if ANIMATABLE.knows(&path) {
        return Ok(vec![path]);
    }
    let axes: Vec<PropertyPath> = ["x", "y"]
        .iter()
        .map(|axis| PropertyPath::new(format!("{asked}.{axis}")))
        .collect();
    if axes.iter().all(|axis| ANIMATABLE.knows(axis)) {
        return Ok(axes);
    }
    let hint = ANIMATABLE.did_you_mean(&path).map_or_else(
        || ".".to_owned(),
        |known| format!(" — did you mean `{}`?", known.path),
    );
    Err(format!(
        "nothing in this build animates `{asked}`{hint} The animatable properties are \
         the table in `guide project-format`, section *What the compositor animates today*; \
         nothing was written"
    ))
}

/// One keyframe as asked for, before and after it meets the frame grid.
#[derive(Debug, Clone, Copy)]
pub(super) struct Point {
    /// When, in seconds from the clip's start, as given.
    pub(super) seconds: f64,
    /// The same moment on the project's grid.
    pub(super) frame: Frames,
    /// What the property reads there.
    pub(super) value: f64,
    /// How it travels to the next point.
    pub(super) easing: Easing,
}

impl Point {
    /// The keyframe the document holds for this point.
    pub(super) fn keyframe(self) -> Keyframe {
        Keyframe {
            t: self.frame,
            value: self.value,
            easing: self.easing,
        }
    }
}

// One entry of `keyframes`, as a client sends it. Its doc is a plain comment
// on purpose: a doc comment would become a description on the array's items,
// which the schema has never carried.
#[derive(Deserialize, JsonSchema)]
pub(super) struct Asked {
    /// When, in seconds from the start of the CLIP (not the timeline), rounded
    /// to the project's frame grid. At most the clip's length.
    at_seconds: f64,
    /// What the property reads at that moment.
    value: f64,
    /// How it travels from here to the next point: `linear` (the default),
    /// `ease_in`, `ease_out`, `ease_in_out`, `hold`, `back_in`, `back_out` (a
    /// pop that passes its mark and settles), `back_in_out`, `spring`, or {
    /// "cubic_bezier": [x1, y1, x2, y2] } as in CSS.
    //
    // JSON rather than a type: a name or an object, read by the format's own
    // `Easing`, whose refusal names the shapes it takes.
    #[serde(default)]
    #[schemars(schema_with = "untyped")]
    easing: Option<Value>,
}

/// A schema that says nothing about the type, for an argument of more than one
/// shape whose description says which.
fn untyped(_: &mut SchemaGenerator) -> Schema {
    json_schema!({})
}

/// The `keyframes` argument, read, put in time order and placed on the grid.
///
/// Sorted rather than refused when out of order, since each point's easing
/// travels with it and the order of a list in a call carries no meaning of
/// its own. Two points that land on one frame are refused: one of them would
/// have to be dropped, and which is not this tool's to choose.
pub(super) fn points(listed: &[Asked], fps: Fps) -> Result<Vec<Point>, String> {
    let mut points = listed
        .iter()
        .enumerate()
        .map(|(index, entry)| point(index, entry, fps))
        .collect::<Result<Vec<_>, _>>()?;
    points.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    if let Some(pair) = points
        .windows(2)
        .find(|pair| pair[0].frame == pair[1].frame)
    {
        return Err(format!(
            "keyframes at {}s and {}s both land on frame {} of the clip at this frame \
             rate — move one, or drop it; nothing was written",
            pair[0].seconds,
            pair[1].seconds,
            pair[0].frame.get()
        ));
    }
    Ok(points)
}

/// One entry of the list, refused by its position when it is not a keyframe.
fn point(index: usize, entry: &Asked, fps: Fps) -> Result<Point, String> {
    let at = |problem: String| format!("keyframe {index}: {problem}");
    let seconds = seconds(Some(entry.at_seconds), "at_seconds")
        .map_err(at)?
        .unwrap_or_default();
    let easing = match &entry.easing {
        None => Easing::Linear,
        Some(name) => serde_json::from_value(name.clone()).map_err(|_| {
            at(format!(
                "`easing` {name} is not a keyframe easing — a name such as `ease_out` or \
                 `back_out`, or {{ \"cubic_bezier\": [x1, y1, x2, y2] }}"
            ))
        })?,
    };
    Ok(Point {
        seconds,
        frame: fps.frames(seconds),
        value: entry.value,
        easing,
    })
}
/// An easing as the document spells it, for a reply: `back_out`, or the
/// bezier's object.
pub(super) fn spelled(easing: Easing) -> String {
    match serde_json::to_value(easing) {
        Ok(Value::String(name)) => name,
        Ok(other) => other.to_string(),
        Err(_) => format!("{easing:?}"),
    }
}
