//! What a tile shows when its asset has no picture to take: a drawing of what
//! kind of thing it is, in the muted ink the web draws it in.

use egui::{Align2, Color32, Painter, Rect, Stroke, StrokeKind, TextStyle, Ui, pos2, vec2};
use scorsese_core::AssetKind;

use crate::theme::ROUND_SM;

/// Draws the glyph for `kind` centred in `area`.
pub(super) fn draw(ui: &Ui, painter: &Painter, area: Rect, kind: AssetKind, hue: Color32) {
    let centre = area.center();
    let unit = area.height() * 0.22;
    let stroke = Stroke::new(2.0, hue);
    match kind {
        AssetKind::Audio | AssetKind::GeneratedAudio | AssetKind::SynthAudio => {
            // A waveform: bars rising and falling about the middle.
            const BARS: [f32; 9] = [0.3, 0.6, 0.9, 0.5, 1.0, 0.7, 0.9, 0.4, 0.25];
            let step = unit * 0.5;
            let left = centre.x - step * (BARS.len() as f32 - 1.0) / 2.0;
            for (n, height) in BARS.iter().enumerate() {
                let x = left + step * n as f32;
                painter.line_segment(
                    [
                        pos2(x, centre.y - unit * height),
                        pos2(x, centre.y + unit * height),
                    ],
                    stroke,
                );
            }
        }
        AssetKind::Text => words(ui, painter, centre, "Aa", hue),
        AssetKind::Html => words(ui, painter, centre, "</>", hue),
        AssetKind::Color => {
            painter.rect_filled(
                Rect::from_center_size(centre, vec2(unit * 2.4, unit * 1.6)),
                ROUND_SM,
                hue,
            );
        }
        AssetKind::Shape => {
            painter.rect_stroke(
                Rect::from_center_size(centre, vec2(unit * 1.6, unit * 1.6)),
                ROUND_SM,
                stroke,
                StrokeKind::Middle,
            );
        }
        AssetKind::Icon => {
            painter.circle_stroke(centre, unit * 0.9, stroke);
        }
        AssetKind::Group => {
            let size = vec2(unit * 1.5, unit * 1.2);
            let shift = vec2(unit * 0.4, unit * 0.3);
            painter.rect_stroke(
                Rect::from_center_size(centre - shift, size),
                ROUND_SM,
                stroke,
                StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_center_size(centre + shift, size),
                ROUND_SM,
                stroke,
                StrokeKind::Middle,
            );
        }
        // Footage and stills: the picture is on its way, or could not be made.
        _ => {}
    }
}

fn words(ui: &Ui, painter: &Painter, at: egui::Pos2, text: &str, hue: Color32) {
    let font = TextStyle::Heading.resolve(ui.style());
    painter.text(at, Align2::CENTER_CENTER, text, font, hue);
}
