//! Drawing one lane: its ground, the clips along it, and the head beside them.
//!
//! Everything here paints and nothing here decides: which clip is under the
//! pointer is [`super::hit`]'s question, answered from the same
//! [`clip::rect`] the drawing uses, so a block can never be drawn where it
//! cannot be grabbed.

mod clip;
mod gutter;

use std::collections::BTreeSet;

use egui::{Painter, Rect};
use scorsese_core::{AssetId, ClipId, Project, Track};

use crate::theme::{ROUND, marks, palette};
use crate::timeline::view::View;

pub(in crate::timeline::lanes) use clip::rect as clip_rect;
pub(in crate::timeline) use gutter::draw as gutter;

/// Everything drawing a lane needs that is the same for every lane.
///
/// Bundled rather than passed one by one: these travel together through every
/// function here, and a signature long enough to need counting is one nobody
/// reads.
///
/// **No `Ui` on it**, and that is the theme showing through rather than an
/// omission. This used to carry one for the single purpose of reading colours
/// off `ui.visuals()`; every colour now comes from `crate::theme::palette`, and
/// a painter needs nothing from a layout.
pub(in crate::timeline) struct Paint<'a> {
    /// Where the shapes go.
    pub(in crate::timeline) painter: &'a Painter,
    /// The document, for resolving a clip's asset.
    pub(in crate::timeline) project: &'a Project,
    /// How far in and how magnified.
    pub(in crate::timeline) view: View,
    /// The selected clips. Borrowed rather than copied: the set is read once
    /// per clip drawn and a timeline redraws while a hand is moving.
    pub(in crate::timeline) selected: &'a BTreeSet<ClipId>,
    /// An asset picked out in the files panel. Its clips are drawn at full
    /// strength and everything else steps back, which answers "where is this
    /// used?" without anyone having to read ids off a timeline.
    pub(in crate::timeline) highlighted: Option<AssetId>,
}

/// One lane's ground.
///
/// Apart from the clips because the grid goes between them: a vertical rule
/// drawn over a lane and under its blocks shows in the gaps, which is where
/// somebody looking to see whether two cuts line up is looking. Drawn over the
/// blocks it would be a stripe through every label on the timeline.
pub(in crate::timeline) fn ground(painter: &Painter, lane: Rect) {
    painter.rect_filled(lane, ROUND, palette::RAISED);
    // A hairline under each lane rather than a gap of a different colour: the
    // lanes are a stack of like things, and a rule is how a stack of like
    // things is divided without implying that any two of them are grouped.
    marks::rule(
        painter,
        lane.left_bottom(),
        lane.right_bottom(),
        palette::RULE,
    );
}

/// The clips along one lane.
pub(in crate::timeline) fn draw(paint: &Paint<'_>, lane: Rect, track: &Track) {
    let Paint { view, .. } = *paint;
    for held in &track.clips {
        let rect = clip::rect(lane, held, view);
        // A clip scrolled off the edge is not drawn at all: at a wide zoom a
        // long project is thousands of clips, and painting the ones nobody can
        // see is work the scrub loop cannot afford.
        if rect.right() < lane.left() || rect.left() > lane.right() {
            continue;
        }
        clip::draw(paint, rect, lane, held);
    }
}
