//! The few lucide-style icons the window draws.
//!
//! Painted, not typed: Geist has none of these symbols, and a symbol no face
//! has draws as an empty box. Each is a line drawing in a 16-unit box, at the
//! web's 1.5 stroke, scaled to the size asked for.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};

/// What an icon shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    /// `SkipBackIcon`: to the start.
    Start,
    /// `StepBackIcon`: one frame back.
    StepBack,
    Play,
    Pause,
    /// `StepForwardIcon`: one frame on.
    StepForward,
    /// `SkipForwardIcon`: to the end.
    End,
    /// `FilmIcon`: Render.
    Film,
    /// `FolderOpenIcon`: Open.
    Folder,
    /// `SparklesIcon`: Generate.
    Sparkle,
}

impl Icon {
    /// Draws the icon centred in `area`, a square.
    pub(crate) fn paint(self, painter: &Painter, area: Rect, colour: Color32) {
        let side = area.width().min(area.height());
        let unit = side / 16.0;
        let at = |x: f32, y: f32| area.center() + vec2(x - 8.0, y - 8.0) * unit;
        let stroke = Stroke::new((1.5 * unit).max(1.0), colour);
        let line = |points: &[(f32, f32)]| {
            let points: Vec<Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
            painter.add(Shape::line(points, stroke));
        };
        let solid = |points: &[(f32, f32)]| {
            let points: Vec<Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
            painter.add(Shape::convex_polygon(points, colour, stroke));
        };
        match self {
            Self::Start => {
                line(&[(3.0, 3.0), (3.0, 13.0)]);
                solid(&[(13.0, 3.0), (6.0, 8.0), (13.0, 13.0)]);
            }
            Self::StepBack => {
                line(&[(13.0, 3.0), (13.0, 13.0)]);
                solid(&[(11.0, 3.0), (4.0, 8.0), (11.0, 13.0)]);
            }
            Self::Play => solid(&[(4.0, 2.5), (13.0, 8.0), (4.0, 13.5)]),
            Self::Pause => {
                solid(&[(4.0, 3.0), (6.5, 3.0), (6.5, 13.0), (4.0, 13.0)]);
                solid(&[(9.5, 3.0), (12.0, 3.0), (12.0, 13.0), (9.5, 13.0)]);
            }
            Self::StepForward => {
                line(&[(3.0, 3.0), (3.0, 13.0)]);
                solid(&[(5.0, 3.0), (12.0, 8.0), (5.0, 13.0)]);
            }
            Self::End => {
                line(&[(13.0, 3.0), (13.0, 13.0)]);
                solid(&[(3.0, 3.0), (10.0, 8.0), (3.0, 13.0)]);
            }
            Self::Film => {
                painter.rect_stroke(
                    Rect::from_two_pos(at(2.0, 2.5), at(14.0, 13.5)),
                    2.0 * unit,
                    stroke,
                    egui::StrokeKind::Middle,
                );
                line(&[(5.5, 2.5), (5.5, 13.5)]);
                line(&[(10.5, 2.5), (10.5, 13.5)]);
                line(&[(2.0, 8.0), (14.0, 8.0)]);
            }
            Self::Folder => {
                line(&[
                    (2.0, 12.5),
                    (2.0, 3.5),
                    (6.0, 3.5),
                    (7.5, 5.0),
                    (13.0, 5.0),
                    (13.0, 7.0),
                ]);
                line(&[
                    (2.0, 12.5),
                    (4.0, 7.0),
                    (14.5, 7.0),
                    (12.5, 12.5),
                    (2.0, 12.5),
                ]);
            }
            Self::Sparkle => {
                line(&[(8.0, 2.0), (9.3, 6.7), (14.0, 8.0), (9.3, 9.3), (8.0, 14.0)]);
                line(&[(8.0, 14.0), (6.7, 9.3), (2.0, 8.0), (6.7, 6.7), (8.0, 2.0)]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every icon can be drawn at the sizes the window asks for, down to a
    /// degenerate one a squeezed button may hand it.
    #[test]
    fn every_icon_draws_at_any_size() {
        let ctx = egui::Context::default();
        let painter = Painter::new(ctx, egui::LayerId::background(), Rect::EVERYTHING);
        for icon in [
            Icon::Start,
            Icon::StepBack,
            Icon::Play,
            Icon::Pause,
            Icon::StepForward,
            Icon::End,
            Icon::Film,
            Icon::Folder,
            Icon::Sparkle,
        ] {
            for side in [0.0, 1.0, 14.0, 16.0] {
                let area = Rect::from_center_size(egui::pos2(50.0, 50.0), vec2(side, side));
                icon.paint(&painter, area, Color32::BLACK);
            }
        }
    }
}
