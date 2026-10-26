//! One tile of the files panel: a picture, the kind's colour as its edge, the
//! name under it — the web editor's Assets sidebar, drawn with the app's theme.

use egui::text::LayoutJob;
use egui::{CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use egui::{Response, TextureHandle};
use scorsese_core::AssetKind;

use super::glyph;
use crate::theme::{ROUND_LG, marks, palette};

/// The picture's shape: the web's tile is 16:9 as well.
const RATIO: f32 = 9.0 / 16.0;
/// How thick the kind's colour edge is: the web's `p-px`.
const EDGE: f32 = 1.0;
/// What the arrow that unfolds a sequence measures.
const ARROW: f32 = 20.0;

/// What a tile says about itself beyond its picture.
pub(super) struct Face<'a> {
    pub(super) name: &'a str,
    pub(super) kind: AssetKind,
    /// Picked: its clips are lit on the timeline.
    pub(super) picked: bool,
    /// The one line of state worth saying — absent when all is well — and
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

/// Draws a tile `width` wide: the web's `TileView`, a 1px edge in the kind's
/// hue at `rounded-lg`, the picture inside it, the name under the picture
/// (xs medium) and the state on the line below (11px muted).
pub(super) fn show(ui: &mut Ui, width: f32, face: &Face<'_>) -> Drawn {
    let colours = palette::of(ui.ctx());
    let hue = palette::of_kind(face.kind);
    let name_font = crate::theme::medium_font(ui.ctx(), 12.0);
    let state_font = marks::state_font();
    let picture = (width - 2.0) * RATIO;
    let name = ui.fonts_mut(|fonts| fonts.row_height(&name_font)) + 3.0;
    let state = face.note.as_ref().map_or(0.0, |_| {
        ui.fonts_mut(|fonts| fonts.row_height(&state_font)) + 2.0
    });
    let (rect, tile) = ui.allocate_exact_size(
        vec2(width, picture + name + state + 2.0 + 3.0),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    let inside = rect.shrink(EDGE);
    let area = Rect::from_min_size(inside.min, vec2(inside.width(), picture));
    let round = CornerRadius::same(ROUND_LG.nw.saturating_sub(1));
    let top = CornerRadius {
        sw: 0,
        se: 0,
        ..round
    };

    let ground = if face.picked {
        colours.pressed
    } else if tile.hovered() {
        colours.hover
    } else {
        colours.background
    };
    painter.rect_filled(inside, round, ground);
    if let Some(texture) = face.picture {
        painter.rect_filled(area, top, palette::MATTE);
        let fit = texture.size_vec2();
        let scale = (area.width() / fit.x).min(area.height() / fit.y);
        let shown = Rect::from_center_size(area.center(), fit * scale);
        let rounded = if shown == area {
            top
        } else {
            CornerRadius::ZERO
        };
        egui::Image::new((texture.id(), fit))
            .corner_radius(rounded)
            .paint_at(ui, shown);
    } else {
        painter.rect_filled(area, top, colours.muted);
        if face.unmade {
            // The timeline's hatch: the hue at 15%, 2px stripes every 9px at 45%.
            painter.rect_filled(area, top, palette::over(hue, colours.muted, 0.15));
            marks::hatch(
                &painter.with_clip_rect(area),
                area,
                hue.gamma_multiply(0.45),
                9.0,
                2.0,
            );
        }
        glyph::draw(ui, &painter, area, face.kind, colours.muted_foreground);
    }
    painter.rect_stroke(rect, ROUND_LG, Stroke::new(1.0, hue), StrokeKind::Inside);

    let truncated = |text: &str, font: FontId, colour| {
        let mut job = LayoutJob::simple_singleline(text.to_owned(), font, colour);
        job.wrap.max_width = width - 8.0;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.wrap.overflow_character = Some('…');
        painter.layout_job(job)
    };
    let galley = truncated(face.name, name_font, colours.foreground);
    painter.galley(
        pos2(inside.left() + 3.0, area.bottom() + 2.0),
        galley,
        colours.foreground,
    );
    if let Some((text, fault)) = &face.note {
        let tint = if *fault {
            colours.warning
        } else {
            colours.muted_foreground
        };
        let galley = truncated(text, state_font, tint);
        painter.galley(
            pos2(inside.left() + 3.0, area.bottom() + name),
            galley,
            tint,
        );
    }

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
