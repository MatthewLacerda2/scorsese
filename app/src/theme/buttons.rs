//! The web's `Button`: its variants and sizes, drawn with an optional icon.
//!
//! egui's own button cannot carry a painted icon, and the web puts one beside
//! the word in its header and icons alone in its transport, so the two shapes
//! are drawn here from the same `outline`, `ghost` and `default` fills.

use egui::{Align2, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, vec2};

use super::icons::Icon;
use super::{ROUND_LG, palette};

/// Which of the web's variants a button wears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Look {
    /// A hairline and the control fill: the header's actions.
    Outline,
    /// Nothing at rest, `muted` under the pointer: the transport.
    Ghost,
    /// The primary fill: the one main action.
    Primary,
}

/// `size-8`, and `h-8` for the words.
const DEFAULT: f32 = 32.0;
/// `h-7`: the header's `sm`.
const SMALL: f32 = 28.0;
/// The gap between an icon and its word (`gap-1.5` at `sm`, `gap-1`).
const GAP: f32 = 4.0;
/// `px-2.5`.
const PAD: f32 = 10.0;

/// An icon on its own, `size-8`.
pub(crate) fn icon(ui: &mut Ui, look: Look, icon: Icon, hint: &str) -> Response {
    draw(ui, look, true, Some(icon), "", DEFAULT).on_hover_text(hint)
}

/// An `sm` button: an icon and a word, or the word alone.
pub(crate) fn small(
    ui: &mut Ui,
    enabled: bool,
    look: Look,
    icon: Option<Icon>,
    text: &str,
) -> Response {
    draw(ui, look, enabled, icon, text, SMALL)
}

fn draw(
    ui: &mut Ui,
    look: Look,
    enabled: bool,
    icon: Option<Icon>,
    text: &str,
    height: f32,
) -> Response {
    let colours = palette::of(ui.ctx());
    let font = TextStyle::Button.resolve(ui.style());
    let ink = if look == Look::Primary {
        colours.primary_foreground
    } else {
        colours.foreground
    };
    let galley =
        (!text.is_empty()).then(|| ui.painter().layout_no_wrap(text.to_owned(), font, ink));
    let glyph = if height == SMALL { 14.0 } else { 16.0 };
    let mut content = galley.as_ref().map_or(0.0, |galley| galley.size().x);
    if icon.is_some() {
        content += glyph;
        if galley.is_some() {
            content += GAP;
        }
    }
    let width = if galley.is_none() {
        height
    } else {
        content + 2.0 * PAD
    };
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let (fill, stroke) = match look {
        Look::Outline => (
            if pressed {
                colours.pressed
            } else if hovered {
                colours.hover
            } else {
                colours.field
            },
            Stroke::new(1.0, colours.input),
        ),
        Look::Ghost => (
            if pressed {
                colours.pressed
            } else if hovered {
                colours.muted
            } else {
                egui::Color32::TRANSPARENT
            },
            Stroke::NONE,
        ),
        Look::Primary => (
            if hovered {
                colours.primary.gamma_multiply(0.8)
            } else {
                colours.primary
            },
            Stroke::NONE,
        ),
    };
    let painter = ui.painter();
    painter.rect(rect, ROUND_LG, fill, stroke, StrokeKind::Inside);
    let ink = if enabled {
        ink
    } else {
        ink.gamma_multiply(0.5)
    };
    let mut x = rect.center().x - content / 2.0;
    if let Some(icon) = icon {
        let area = egui::Rect::from_min_size(
            egui::pos2(x, rect.center().y - glyph / 2.0),
            vec2(glyph, glyph),
        );
        icon.paint(painter, area, ink);
        x += glyph + GAP;
    }
    if let Some(galley) = galley {
        let at = egui::pos2(x, rect.center().y);
        painter.galley(
            Align2::LEFT_CENTER.anchor_size(at, galley.size()).min,
            galley,
            ink,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text.to_owned())
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The web's sizes: an icon button is `size-8`, an `sm` button is `h-7`
    /// and as wide as its word, icon and padding need.
    #[test]
    fn buttons_are_the_webs_sizes() {
        let ctx = egui::Context::default();
        let mut sizes = Vec::new();
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |ui| {
                crate::theme::apply(ui.ctx(), crate::theme::choice::Choice::DEFAULT);
                sizes.clear();
                sizes.push(icon(ui, Look::Ghost, Icon::Play, "").rect.size());
                sizes.push(small(ui, true, Look::Outline, None, "Render…").rect.size());
                sizes.push(
                    small(ui, true, Look::Outline, Some(Icon::Film), "Render…")
                        .rect
                        .size(),
                );
            });
        }
        assert_eq!(sizes[0], vec2(32.0, 32.0));
        assert_eq!(sizes[1].y, 28.0);
        assert_eq!(
            sizes[2].x - sizes[1].x,
            14.0 + GAP,
            "an icon adds its own width and the gap"
        );
    }
}
