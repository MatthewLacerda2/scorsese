//! A clip travelling along a path: the arrow it follows, and whether it turns
//! to face the way the arrow runs.
//!
//! **The path is an arrow clip's, never an arrow asset's.** An arrow with an
//! attached end is only resolved per frame and per clip — where it runs depends
//! on where the clips it points at have got to — and one asset can be on screen
//! twice at once. So the one thing that names a single line on the frame is a
//! clip, and that is what a follower names.
//!
//! **How far along is animated, not stored.** `follow.progress`, a keyframe
//! track like `transform.position.x`, says where on the arrow the clip is —
//! `0.0` its tail, `1.0` its head, by distance along the line rather than the
//! curve's own parameter, so an S-shaped connector is travelled at an even
//! pace. Nothing here holds a number for it, for the reason trim is keyframe
//! only (#583): a held value is one keyframe, and a field beside a track is two
//! places for one fact.

use serde::{Deserialize, Serialize};

use super::ClipId;

/// The arrow a clip travels along, and whether it turns with it.
///
/// What it does to the clip's own transform, decided once here and in
/// `docs/project-format.md`:
///
/// - The path places the clip's **origin point** — the point of its content
///   its transform turns about, the centre unless [`super::Origin`] says
///   otherwise. The dot's middle rides the line, not its corner.
/// - `transform.position` is then an **offset** from that point, in the same
///   fractions of the frame it always is — so a small bob keyframed on top
///   rides along with the path instead of replacing it.
/// - [`Follow::orient`] **adds** the line's heading to `transform.rotation`,
///   so a clip drawn pointing right faces along the line, and a rotation track
///   still turns it on top of that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    /// The arrow clip whose line is the path. It must be a clip of an arrow
    /// shape, on the same timeline as the follower — both inside one group, or
    /// both outside every group — and not the follower itself.
    ///
    /// The arrow need not be on screen: its line is geometry, and a dot may set
    /// off along a connector before the connector has drawn itself in. An arrow
    /// with an attached end is the exception, since where it runs is known only
    /// while it is drawn; off screen, its follower is left out and the render
    /// says so.
    pub clip: ClipId,
    /// Turn the clip to face along the line, adding the line's heading at the
    /// clip's place on it to `transform.rotation`. Off — and absent — is a
    /// clip that slides along the line facing however it was drawn.
    #[serde(default, skip_serializing_if = "is_false")]
    pub orient: bool,
}

impl Follow {
    /// Following `clip`, facing however the follower was drawn.
    pub fn new(clip: ClipId) -> Self {
        Self {
            clip,
            orient: false,
        }
    }
}

/// Whether `orient` is off, so it can be left out of the document.
fn is_false(orient: &bool) -> bool {
    !*orient
}
