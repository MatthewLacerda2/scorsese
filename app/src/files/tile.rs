//! One tile of the files panel: a picture, the kind's colour as its edge, the
//! name under it — the web editor's Assets sidebar, drawn with the app's theme.

use egui::text::LayoutJob;
use egui::{Rect, Sense, Stroke, StrokeKind, TextStyle, Ui, pos2, vec2};
use egui::{Response, TextureHandle};
use scorsese_core::AssetKind;

use super::glyph;
use crate::theme::{ROUND_SM, marks, palette};

/// The picture's shape: the web's tile is 16:9 as well.
const RATIO: f32 = 9.0 / 16.0;
/// How thick the kind's colour edge is.
const EDGE: f32 = 2.0;
/// What the arrow that unfolds a sequence measures.
const ARROW: f32 = 20.0;

/// What a tile says about itself beyond its picture.
pub(super) struct Face<'a> {
    pub(super) name: &'a str,
    pub(super) kind: AssetKind,
    /// Picked: its clips are lit on the timeline.
    pub(super) picked: bool,
    /// The one word of state worth a tag — absent when all is well — and
    /// whether it is a fault rather than a fact.
    pub(super) note: Option<(String, bool)>,
    /// Drawn hatched, like its unmade clip.
    pub(super) unmade: bool,
    pub(super) picture: Option<&'a TextureHandle>,
    /// For a sequence: whether it is unfolded. `None` for everything else.
    pub(super) unfolded: Option<bool>,
}

/// What came of drawing one.
pub(super) struct Drawn {
    pub(super) tile: Response,
    pub(super) arrow_clicked: bool,
}

/// Draws a tile `width` wide.
pub(super) fn show(ui: &mut Ui, width: f32, face: &Face<'_>) -> Drawn {
    let label = ui.text_style_height(&TextStyle::Small) + 6.0;
    let picture = width * RATIO;
    let (rect, tile) =
        ui.allocate_exact_size(vec2(width, picture + label), Sense::click_and_drag());
    let colours = palette::of(ui.ctx());
    let hue = palette::of_kind(face.kind);
    let painter = ui.painter_at(rect);
    let area = Rect::from_min_size(rect.min, vec2(width, picture));

    if face.picked || tile.hovered() {
        painter.rect_filled(
            rect,
            ROUND_SM,
            if face.picked {
                colours.pressed
            } else {
                colours.hover
            },
        );
    }
    if let Some(texture) = face.picture {
        painter.rect_filled(area, ROUND_SM, palette::MATTE);
        let fit = texture.size_vec2();
        let scale = (area.width() / fit.x).min(area.height() / fit.y);
        let shown = Rect::from_center_size(area.center(), fit * scale);
        painter.image(
            texture.id(),
            shown,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            palette::UNTINTED,
        );
    } else {
        painter.rect_filled(area, ROUND_SM, colours.muted);
        if face.unmade {
            painter.rect_filled(area, ROUND_SM, palette::over(hue, colours.background, 0.14));
            marks::hatch(
                &painter.with_clip_rect(area),
                area,
                hue.gamma_multiply(0.5),
                8.0,
            );
        } else {
            glyph::draw(ui, &painter, area, face.kind);
        }
    }
    painter.rect_stroke(area, ROUND_SM, Stroke::new(EDGE, hue), StrokeKind::Inside);
    if let Some((text, fault)) = &face.note {
        let tint = if *fault {
            colours.warning
        } else {
            colours.foreground
        };
        marks::tag(&painter, area.min + vec2(4.0, 4.0), text, tint);
    }

    let mut job = LayoutJob::simple_singleline(
        face.name.to_owned(),
        TextStyle::Small.resolve(ui.style()),
        colours.foreground,
    );
    job.wrap.max_width = width - 6.0;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    painter.galley(
        pos2(rect.left() + 3.0, area.bottom() + 3.0),
        galley,
        colours.foreground,
    );

    let mut arrow_clicked = false;
    if let Some(open) = face.unfolded {
        let spot = Rect::from_min_size(
            area.max - vec2(ARROW + 3.0, ARROW + 3.0),
            vec2(ARROW, ARROW),
        );
        let arrow = ui
            .put(
                spot,
                egui::Button::new(if open { "⏷" } else { "⏵" }).small(),
            )
            .on_hover_text(if open {
                "Fold the photos away"
            } else {
                "Show its photos"
            });
        arrow_clicked = arrow.clicked();
    }
    Drawn {
        tile,
        arrow_clicked,
    }
}
