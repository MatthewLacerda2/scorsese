//! The marks: the handful of shapes and text treatments that make the window
//! look like one window.
//!
//! Small on purpose. Each of these was drawn in two or three places before it
//! lived here, and every one of them is a thing a panel would otherwise invent
//! its own version of — a heading, a tag, a line, a way of saying *this is not
//! finished*.

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Stroke, Ui, pos2, vec2};

use super::{ROUND, palette};

/// How far apart the letters of a heading are set.
///
/// The whole of what makes small capitals read as a *label over a region*
/// rather than as a very short sentence. Two pixels at 10.5pt is about a fifth
/// of an em, which is the classical figure for spacing capitals.
const TRACKING: f32 = 2.0;

/// A section heading: small capitals, letterspaced, in the cool accent.
///
/// Capitals rather than sentence case because these name *regions* and not
/// things — `INSPECTOR` is a label on a wall, `Inspector` is a word somebody
/// wrote. Accent rather than white because a heading is the one text in a panel
/// that nobody reads for its content: you find it, then look under it.
pub(crate) fn heading(label: &str) -> RichText {
    RichText::new(label.to_uppercase())
        .small()
        .strong()
        .extra_letter_spacing(TRACKING)
        .color(palette::ACCENT)
}

/// The same, for a heading *inside* a panel that already has one — a group of
/// rows in the pool, the animated properties on a clip.
///
/// Dim rather than accent: two accents in one column is two things claiming to
/// be the top of the hierarchy.
pub(crate) fn subheading(label: &str) -> RichText {
    RichText::new(label.to_uppercase())
        .small()
        .strong()
        .extra_letter_spacing(TRACKING)
        .color(palette::DIM)
}

/// A heading with a hairline running out from it to the right edge.
///
/// The rule is what makes the heading a *lid* on the rows beneath rather than
/// the first of them. It runs to the edge rather than under the text, so the
/// eye reads the word and then follows the line across the panel.
pub(crate) fn section(ui: &mut Ui, label: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let text = ui.label(heading(label));
        let line = Rect::from_min_max(
            pos2(text.rect.right() + 8.0, text.rect.center().y),
            pos2(ui.max_rect().right(), text.rect.center().y),
        );
        if line.width() > 0.0 {
            ui.painter()
                .line_segment([line.min, line.max], Stroke::new(1.0, palette::RULE));
        }
    });
    ui.add_space(1.0);
}

/// A number, a code, a timecode: anything read digit by digit.
pub(crate) fn figure(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace().color(palette::TEXT)
}

/// The same, for a figure that is context rather than content.
pub(crate) fn figure_dim(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace().small().color(palette::DIM)
}

/// Corner marks around a rectangle: four right angles, no sides.
///
/// The single most borrowed thing in this whole look, and the reason it is
/// worth borrowing is that it frames something **without enclosing it**. A box
/// around a picture competes with the picture's own edge; four ticks at the
/// corners say "this region" and then get out of the way. Used around the
/// preview, and nowhere it would merely be decoration.
pub(crate) fn corners(painter: &Painter, rect: Rect, arm: f32, stroke: Stroke) {
    // A mark longer than the thing it marks is not a corner, it is a border
    // drawn badly.
    let arm = arm.min(rect.width() / 3.0).min(rect.height() / 3.0);
    if arm <= 1.0 {
        return;
    }
    for (corner, along, down) in [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.left_bottom(), 1.0, -1.0),
        (rect.right_bottom(), -1.0, -1.0),
    ] {
        painter.line_segment([corner, corner + vec2(arm * along, 0.0)], stroke);
        painter.line_segment([corner, corner + vec2(0.0, arm * down)], stroke);
    }
}

/// Diagonal hatching inside a rectangle: what *not made yet* looks like.
///
/// A clip with nothing behind it renders as a slug card and costs nothing, and
/// which clips those are is the question somebody asks before pressing a button
/// that spends money. The old answer was a one-pixel outline, which is
/// invisible at any zoom where you can see the whole cut. Hatching is legible
/// across the room and is the drafting convention for *this region is
/// indicated, not drawn* — which is exactly what a brief is.
pub(crate) fn hatch(painter: &Painter, rect: Rect, colour: Color32, spacing: f32) {
    if rect.width() < 1.0 || rect.height() < 1.0 || spacing <= 0.0 {
        return;
    }
    // Only the strokes anybody can see. A clip is drawn at its whole width even
    // when most of it is scrolled off the side, and at a deep zoom that width is
    // tens of thousands of pixels — several thousand line segments rebuilt on
    // every repaint, for a pattern nobody is looking at. The window the caller's
    // painter is already clipped to is the honest bound.
    let seen = rect.intersect(painter.clip_rect());
    if seen.width() < 1.0 {
        return;
    }
    let painter = painter.with_clip_rect(seen);
    let stroke = Stroke::new(1.0, colour);
    // Leaning the same way as the reading direction, and stepped from the
    // rectangle's **own** left edge rather than from the visible one — the
    // phase has to belong to the clip, or the pattern swims through a block
    // being dragged instead of travelling with it.
    let lean = rect.height();
    let first = (((seen.left() - lean) - rect.left()) / spacing)
        .floor()
        .max(0.0);
    let mut x = rect.left() + first * spacing;
    while x <= seen.right() {
        painter.line_segment([pos2(x, rect.bottom()), pos2(x + lean, rect.top())], stroke);
        x += spacing;
    }
}

/// A small tag: a word or two in a tinted box, for a thing's kind or its state.
///
/// Returns how wide it was, so a caller laying things out along a row can put
/// the next one after it.
pub(crate) fn tag(painter: &Painter, at: Pos2, label: &str, colour: Color32) -> f32 {
    let font = FontId::monospace(9.5);
    let text = painter.layout_no_wrap(label.to_owned(), font, colour);
    let box_size = vec2(text.rect.width() + 8.0, text.rect.height() + 3.0);
    let rect = Rect::from_min_size(at, box_size);
    painter.rect_filled(rect, ROUND, palette::over(colour, palette::INK, 0.18));
    painter.galley(
        rect.center() - text.rect.size() / 2.0,
        text,
        Color32::PLACEHOLDER,
    );
    box_size.x
}

/// A line of text on a plate of its own, hung from its top-right corner.
///
/// For anything the app has to say *over* something it is also drawing — a
/// gesture's readout, a refused edit. Bare text over a timeline is text over
/// whatever the timeline happened to put there; the plate is what makes it
/// legible without anybody having to decide in advance where there is room.
///
/// Returns how tall it was, so a caller stacking two of them knows where the
/// next one goes.
pub(crate) fn plate(painter: &Painter, right_top: Pos2, text: &str, colour: Color32) -> f32 {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), colour);
    let size = galley.rect.size() + vec2(14.0, 5.0);
    let rect = Rect::from_min_size(pos2(right_top.x - size.x, right_top.y), size);
    painter.rect_filled(rect, ROUND, palette::INK);
    painter.rect_stroke(
        rect,
        ROUND,
        Stroke::new(1.0, colour.gamma_multiply(0.45)),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        rect.center() - galley.rect.size() / 2.0,
        galley,
        Color32::PLACEHOLDER,
    );
    size.y
}

/// A hairline between two points.
pub(crate) fn rule(painter: &Painter, from: Pos2, to: Pos2, colour: Color32) {
    painter.line_segment([from, to], Stroke::new(1.0, colour));
}

/// Text at a position, in the one place the whole app's plain-painted text is
/// spelled — so a label on a clip and a label on a lane cannot end up in
/// different fonts by nobody deciding.
pub(crate) fn label(
    painter: &Painter,
    at: Pos2,
    align: Align2,
    text: &str,
    size: f32,
    colour: Color32,
) {
    painter.text(at, align, text, FontId::proportional(size), colour);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A heading is capitals whatever it was handed, because the panels spell
    /// their own names in sentence case and the look is not theirs to decide.
    #[test]
    fn a_heading_is_set_in_capitals() {
        assert_eq!(heading("Inspector").text(), "INSPECTOR");
        assert_eq!(subheading("picture").text(), "PICTURE");
    }

    /// Corner marks on something smaller than the marks would be a box. The
    /// preview is letterboxed into whatever room is left, and a window dragged
    /// narrow enough is a real state rather than a hypothetical one.
    #[test]
    fn corner_marks_never_grow_into_a_border() {
        let ctx = egui::Context::default();
        let painter = Painter::new(
            ctx.clone(),
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0)),
        );
        // Nothing to assert but that it does not panic and does not draw: a
        // rectangle three pixels wide has no room for a twelve-pixel arm.
        corners(
            &painter,
            Rect::from_min_size(Pos2::ZERO, vec2(3.0, 3.0)),
            12.0,
            Stroke::new(1.0, palette::ACCENT),
        );
    }

    /// Hatching a rectangle with no area, one with a nonsense spacing, and one
    /// entirely outside what the painter may draw on. All three are reachable:
    /// a lane can be dragged to nothing, and a clip is drawn at its whole width
    /// however little of it is on screen.
    #[test]
    fn hatching_nothing_draws_nothing() {
        let ctx = egui::Context::default();
        let painter = Painter::new(
            ctx.clone(),
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0)),
        );
        hatch(&painter, Rect::ZERO, palette::ACCENT, 5.0);
        hatch(
            &painter,
            Rect::from_min_size(Pos2::ZERO, vec2(20.0, 20.0)),
            palette::ACCENT,
            0.0,
        );
        hatch(
            &painter,
            Rect::from_min_size(pos2(400.0, 0.0), vec2(50.0, 20.0)),
            palette::ACCENT,
            5.0,
        );
    }
}
