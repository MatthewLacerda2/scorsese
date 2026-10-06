//! An arrow's two ends: where each one is, and how the line between them runs.

use schemars::JsonSchema;
use scorsese_core::{Attach, ClipId, Curve, Endpoint, Heads, Point, Side};
use serde::Deserialize;

use crate::tools::args;

/// One end of an arrow as a call writes it: a fixed point, or a clip it
/// follows. An object rather than four flat arguments, because the two ends
/// take the same four fields and a flattened pair would be `from_x`,
/// `to_side` and six more names to keep straight.
#[derive(Deserialize, JsonSchema)]
pub(super) struct End {
    /// Across, as a fraction of the frame's width from the left edge. Outside
    /// 0-1 is allowed: an arrow may come in from off-screen.
    x: Option<f64>,
    /// Down, as a fraction of the frame's height from the top edge.
    y: Option<f64>,
    /// A clip to follow instead of a fixed point — a CLIP id, not an asset's,
    /// because one asset can be on screen twice at once.
    clip: Option<String>,
    /// Which side of that clip to meet. Default `center`, which is right when
    /// the arrow points AT something rather than touching it.
    side: Option<Meets>,
}

/// Which side of an attached clip the arrow meets.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Meets {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// How the line runs from one end to the other.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(super) enum Line {
    Straight,
    S,
}

/// Which ends carry a head.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(super) enum Tips {
    None,
    End,
    Both,
}

/// One end of an arrow, required: an arrow is its two ends.
pub(super) fn endpoint(given: Option<&End>, key: &str) -> Result<Endpoint, String> {
    let end = given.ok_or_else(|| {
        format!("`{key}` is required: where the arrow {key} — an arrow is its two ends")
    })?;
    if let Some(clip) = args::given(end.clip.as_deref()) {
        let side = match end.side.unwrap_or(Meets::Center) {
            Meets::Left => Side::Left,
            Meets::Right => Side::Right,
            Meets::Top => Side::Top,
            Meets::Bottom => Side::Bottom,
            Meets::Center => Side::Center,
        };
        return Ok(Endpoint::Attached {
            attach: Attach {
                clip: ClipId::new(clip),
                side,
            },
        });
    }
    let at = |value: Option<f64>, axis: &str, what: &str| {
        value.ok_or_else(|| format!("`{axis}` is required: how far {what} the frame `{key}` is"))
    };
    Ok(Endpoint::At(Point::new(
        at(end.x, "x", "across")?,
        at(end.y, "y", "down")?,
    )))
}

impl From<Line> for Curve {
    fn from(line: Line) -> Self {
        match line {
            Line::Straight => Self::Straight,
            Line::S => Self::S,
        }
    }
}

impl From<Tips> for Heads {
    fn from(tips: Tips) -> Self {
        match tips {
            Tips::None => Self::None,
            Tips::End => Self::End,
            Tips::Both => Self::Both,
        }
    }
}

/// The `to` argument's schema: an end like `from`, said once rather than
/// twice — the two take the same four fields, and every model call pays for
/// each word of a listing.
pub(super) fn to_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "object",
        "description": "(shape: an arrow, when making it) Where the arrow ends, head \
                        first — written exactly as `from` is: `x` and `y`, or a `clip` \
                        and its `side`."
    })
}

/// The description of one end, `what` being which end it is.
pub(super) fn endpoint_described(what: &str) -> String {
    format!(
        "(shape: an arrow, when making it) Where the arrow {what}. Either a point on the frame \
         (`x` and `y`) or a clip to follow (`clip`, and which `side` of it). An \
         attached end is resolved on every frame, so the arrow moves when the clip \
         does; a point stays where it was put."
    )
}
