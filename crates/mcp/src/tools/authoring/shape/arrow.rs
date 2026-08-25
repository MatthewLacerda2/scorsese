//! An arrow's two ends: where each one is, and how the line between them runs.

use scorsese_core::{Attach, ClipId, Curve, Endpoint, Heads, Point, Side};
use serde_json::Value;

use crate::tools::authoring::{maybe, required_number};

/// One end of an arrow: a fixed point, or a clip it follows.
pub(super) fn endpoint(arguments: &Value, key: &str) -> Result<Endpoint, String> {
    let end = arguments
        .get(key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| {
            format!("`{key}` is required: where the arrow {key} — an arrow is its two ends")
        })?;
    if let Some(clip) = maybe(end, "clip") {
        return Ok(Endpoint::Attached {
            attach: Attach {
                clip: ClipId::new(clip),
                side: side(end)?,
            },
        });
    }
    Ok(Endpoint::At(Point::new(
        required_number(end, "x", &format!("how far across the frame `{key}` is"))?,
        required_number(end, "y", &format!("how far down the frame `{key}` is"))?,
    )))
}

/// Which side of an attached clip the arrow meets.
fn side(end: &Value) -> Result<Side, String> {
    match maybe(end, "side").as_deref() {
        None | Some("center") => Ok(Side::Center),
        Some("left") => Ok(Side::Left),
        Some("right") => Ok(Side::Right),
        Some("top") => Ok(Side::Top),
        Some("bottom") => Ok(Side::Bottom),
        Some(other) => Err(format!(
            "`side` is left, right, top, bottom or center, not `{other}`"
        )),
    }
}

/// Straight, or bowed into an S.
pub(super) fn curve(arguments: &Value) -> Result<Option<Curve>, String> {
    match maybe(arguments, "curve").as_deref() {
        None => Ok(None),
        Some("straight") => Ok(Some(Curve::Straight)),
        Some("s") => Ok(Some(Curve::S)),
        Some(other) => Err(format!("`curve` is straight or s, not `{other}`")),
    }
}

/// Which ends carry a head.
pub(super) fn heads(arguments: &Value) -> Result<Option<Heads>, String> {
    match maybe(arguments, "heads").as_deref() {
        None => Ok(None),
        Some("none") => Ok(Some(Heads::None)),
        Some("end") => Ok(Some(Heads::End)),
        Some("both") => Ok(Some(Heads::Both)),
        Some(other) => Err(format!("`heads` is none, end or both, not `{other}`")),
    }
}

/// The schema of one end. An object rather than four flat arguments, because
/// the two ends take the same four fields and a flattened pair would be
/// `from_x`, `to_side` and six more names to keep straight.
pub(super) fn endpoint_property(what: &str) -> Value {
    serde_json::json!({
        "type": "object",
        "description": format!(
            "Where the arrow {what} — an `arrow` only. Either a point on the frame \
             (`x` and `y`) or a clip to follow (`clip`, and which `side` of it). An \
             attached end is resolved on every frame, so the arrow moves when the clip \
             does; a point stays where it was put."
        ),
        "properties": {
            "x": { "type": "number",
                "description": "Across, as a fraction of the frame's width from the left \
                                edge. Outside 0-1 is allowed: an arrow may come in from \
                                off-screen." },
            "y": { "type": "number",
                "description": "Down, as a fraction of the frame's height from the top \
                                edge." },
            "clip": { "type": "string",
                "description": "A clip to follow instead of a fixed point — a CLIP id, \
                                not an asset's, because one asset can be on screen \
                                twice at once." },
            "side": { "type": "string",
                "enum": ["left", "right", "top", "bottom", "center"],
                "description": "Which side of that clip to meet. Default `center`, which \
                                is right when the arrow points AT something rather than \
                                touching it." }
        }
    })
}

/// The schema of how the line runs.
pub(super) fn curve_property() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["straight", "s"],
        "description": "How the arrow gets from one end to the other — an `arrow` only. \
                        `straight` is the default; `s` bows it so it leaves and arrives \
                        along the same axis, which is what a connector between two boxes \
                        side by side wants."
    })
}

/// The schema of which ends are pointed.
pub(super) fn heads_property() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["none", "end", "both"],
        "description": "Which ends carry a head — an `arrow` only. `end` is the default \
                        and points at `to`; `none` is a plain connecting line; `both` \
                        says these two are connected, without a direction."
    })
}
