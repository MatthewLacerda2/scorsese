//! The track head: the fixed column of labels the clips scroll under.
//!
//! Fixed on purpose. A timeline whose labels scroll away is one you get lost
//! in, and what the label answers is the question a hand asks before it moves
//! anything: *is this row picture or sound?*
//!
//! It answers it three times over, which sounds like too many and is not. The
//! bar down the left edge answers it at a glance and from the corner of the
//! eye. The tag says which track, by the id the document and an assistant both
//! use — so a sentence about `a2` and a row on screen are findable from each
//! other. The name says what the person editing calls it, which is the only one
//! of the three that means anything to them.

use egui::{Align2, Painter, Rect, pos2, vec2};
use scorsese_core::Track;

use crate::theme::{marks, palette};

/// How wide the colour bar down the edge of a lane is.
const BAR: f32 = 3.0;
/// Half a tag's height, which is what centring one on a lane costs.
///
/// A measured constant rather than a layout: [`marks::tag`] reports its width
/// because that is what a caller placing the *next* thing needs, and asking it
/// for a height as well would be two return values to save one number that the
/// font size already fixes.
const TAG_HALF: f32 = 7.5;

/// Draws one lane's head.
pub(in crate::timeline) fn draw(painter: &Painter, lane: Rect, track: &Track) {
    let hue = palette::of_track(track.kind);
    painter.rect_filled(
        Rect::from_min_size(lane.min, vec2(BAR, lane.height())),
        0.0,
        hue,
    );

    let mut at = pos2(lane.left() + BAR + 7.0, lane.center().y - TAG_HALF);
    // The id in the document's own spelling, upper-cased — `v1` is what the
    // file says and `V1` is what every editor ever made writes on the row.
    at.x += marks::tag(painter, at, &track.id.as_str().to_uppercase(), hue) + 6.0;

    // The name is optional in the format, and a head that fell back to printing
    // the id twice would look like a bug rather than like an unnamed track.
    if let Some(name) = &track.name {
        marks::label(
            painter,
            pos2(at.x, lane.top() + lane.height() / 2.0),
            Align2::LEFT_CENTER,
            name,
            11.0,
            palette::TEXT,
        );
    }
}
