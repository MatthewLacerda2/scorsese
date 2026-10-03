//! The marks: the handful of shapes and text treatments that make the window
//! look like one window.
//!
//! Small on purpose. Each of these was drawn in two or three places before it
//! lived here, and every one of them is a thing a panel would otherwise invent
//! its own version of — a heading, a tag, a line, a way of saying *this is not
//! finished*.

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Stroke, Ui, pos2, vec2};

use super::{ROUND_SM, palette};

/// How far apart the letters of a heading are set: the web's
/// `tracking-wide`, 0.025em, which is a quarter of a pixel at this size —
/// rounded up to one, because a quarter pixel on a desktop is nothing at all.
const TRACKING: f32 = 1.0;

/// A section heading: small capitals, lightly tracked, in the secondary text
/// colour — the web's `text-xs uppercase tracking-wide text-muted-foreground`.
///
/// Capitals because these name *regions* and not things — `INSPECTOR` is a
/// label on a wall. Quiet because a heading is the one text in a panel nobody
/// reads for its content: you find it, then look under it.
pub(crate) fn heading(label: &str) -> RichText {
    RichText::new(label.to_uppercase())
        .small()
        .weak()
        .extra_letter_spacing(TRACKING)
}

/// The same, for a heading *inside* a panel that already has one — a group of
/// rows in the pool, the animated properties on a clip. The web draws both
/// levels alike, and so does this; the rule [`section`] runs out from a
/// panel's own heading is what tells them apart.
pub(crate) fn subheading(label: &str) -> RichText {
    heading(label)
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
            let rule = palette::of(ui.ctx()).border;
            ui.painter()
                .line_segment([line.min, line.max], Stroke::new(1.0, rule));
        }
    });
    ui.add_space(1.0);
}

/// A number, a code, a timecode: anything read digit by digit.
pub(crate) fn figure(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace()
}

/// The same, for a figure that is context rather than content.
pub(crate) fn figure_dim(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace().small().weak()
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
    let ground = palette::of(painter.ctx()).background;
    painter.rect_filled(rect, ROUND_SM, palette::over(colour, ground, 0.18));
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
    painter.rect_filled(rect, ROUND_SM, palette::of(painter.ctx()).card);
    painter.rect_stroke(
        rect,
        ROUND_SM,
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
        hatch(&painter, Rect::ZERO, palette::PLAYHEAD, 5.0);
        hatch(
            &painter,
            Rect::from_min_size(Pos2::ZERO, vec2(20.0, 20.0)),
            palette::PLAYHEAD,
            0.0,
        );
        hatch(
            &painter,
            Rect::from_min_size(pos2(400.0, 0.0), vec2(50.0, 20.0)),
            palette::PLAYHEAD,
            5.0,
        );
    }
}
