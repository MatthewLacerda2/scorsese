//! The track head: the fixed column of labels the clips scroll under.
//!
//! Fixed on purpose. A timeline whose labels scroll away is one you get lost
//! in. Set as the web's lane header is (`Timeline.tsx`): the track's name in
//! xs medium, its kind in 10px muted beneath, and a ghost bin at the right that
//! removes the track — after the confirm that lists its clips.

use egui::{Align2, Painter, Pos2, Rect, Stroke, pos2, vec2};
use scorsese_core::{Track, TrackKind};

use crate::theme::{ROUND_MD, marks, palette};

/// How far in the name stands from the head's left edge: the web's `pl-2`.
const PAD: f32 = 8.0;
/// The bin button's side: the web's icon button.
const BUTTON: f32 = 28.0;

/// The bin's button inside one lane's head.
pub(in crate::timeline) fn trash(head: Rect) -> Rect {
    Rect::from_center_size(
        pos2(head.right() - 2.0 - BUTTON / 2.0, head.center().y),
        vec2(BUTTON, BUTTON),
    )
}

/// The word for a track's kind, as the web's catalogue spells it.
fn kind_word(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
    }
}

/// Draws one lane's head. `pointer` lights the bin when it is over it.
pub(in crate::timeline) fn draw(
    painter: &Painter,
    lane: Rect,
    track: &Track,
    pointer: Option<Pos2>,
) {
    let colours = palette::of(painter.ctx());
    marks::rule(
        painter,
        lane.left_bottom(),
        lane.right_bottom(),
        colours.border,
    );
    let button = trash(lane);
    let room = (button.left() - lane.left() - PAD - 4.0).max(0.0);
    let name = track.name.as_deref().unwrap_or(track.id.as_str());
    let galley = marks::truncated(
        painter,
        name,
        crate::theme::medium_font(painter.ctx(), 12.0),
        colours.foreground,
        room,
    );
    painter.galley(
        pos2(lane.left() + PAD, lane.center().y - 15.0),
        galley,
        colours.foreground,
    );
    marks::label(
        painter,
        pos2(lane.left() + PAD, lane.center().y + 1.0),
        Align2::LEFT_TOP,
        kind_word(track.kind),
        10.0,
        colours.muted_foreground,
    );

    if pointer.is_some_and(|at| button.contains(at)) {
        painter.rect_filled(button, ROUND_MD, colours.hover);
    }
    bin(painter, button.center(), colours.foreground);
}

/// A small bin: lid, handle and body, drawn in lines because the face has no
/// glyph for one.
fn bin(painter: &Painter, c: Pos2, colour: egui::Color32) {
    let stroke = Stroke::new(1.25, colour);
    painter.line_segment([c + vec2(-6.0, -4.5), c + vec2(6.0, -4.5)], stroke);
    painter.line_segment([c + vec2(-2.0, -4.5), c + vec2(-2.0, -7.0)], stroke);
    painter.line_segment([c + vec2(2.0, -4.5), c + vec2(2.0, -7.0)], stroke);
    painter.line_segment([c + vec2(-2.0, -7.0), c + vec2(2.0, -7.0)], stroke);
    painter.add(egui::Shape::line(
        vec![
            c + vec2(-4.5, -4.5),
            c + vec2(-3.8, 6.5),
            c + vec2(3.8, 6.5),
            c + vec2(4.5, -4.5),
        ],
        stroke,
    ));
    painter.line_segment([c + vec2(-1.5, -1.5), c + vec2(-1.5, 3.5)], stroke);
    painter.line_segment([c + vec2(1.5, -1.5), c + vec2(1.5, 3.5)], stroke);
}
