//! The look: one module that decides every colour, weight and spacing.
//!
//! Before this the window was **stock `egui` dark** — nothing in `app/` had
//! ever touched `Visuals` or `text_styles`, so what a person saw was the
//! toolkit's default theme with our panels drawn into it. That is a fine place
//! to start and a poor place to stay: a timeline where a title, a shot and a
//! music bed are three shades of the same grey is one you have to click to
//! read.
//!
//! ## Where it comes from
//!
//! Two references, answering different questions. **Filmora 9** for the
//! timeline, which `CLAUDE.md` already names as the reference for taste: clips
//! coloured by what the media is, a track head that says which row is which, a
//! ruler whose grid continues down through the lanes. **Mark Coleran's film
//! interfaces** for the chrome: near-black grounds, hairline rules, letterspaced
//! small capitals over sections, monospaced numerics, and marks that frame a
//! thing rather than boxes that contain it.
//!
//! The restraint is the point. A screen interface in a film has to read in two
//! seconds from across a cinema and never has to be used; this one has to be
//! used all evening. So the vocabulary is borrowed and the density is not —
//! no scan lines, no ornament, nothing animated that a person did not start.
//!
//! ## Where it is applied
//!
//! From [`Scorsese::draw`](crate::Scorsese::draw), on every repaint, and **not**
//! from `main.rs`. The snapshot harness calls `draw` with no event loop and no
//! window, so a theme installed at startup would be a theme the reference images
//! never see — and the pictures in `app/tests/snapshots/` would be of a window
//! nobody uses.

pub(crate) mod marks;
pub(crate) mod palette;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

/// How much rounding anything gets.
///
/// Two pixels, everywhere, and that is a decision rather than a default. A
/// square corner reads as technical and a round one reads as soft; two pixels is
/// the amount that takes the aliasing off an edge without saying either.
pub(crate) const ROUND: CornerRadius = CornerRadius::same(2);

/// Installs the whole look into `ctx`.
///
/// Idempotent: it writes a fixed style built from `egui`'s default rather than
/// deriving one from what is already installed, so calling it on every repaint
/// cannot compound. The cost is one `Style` and two `Arc`s a frame, and it buys
/// having no second place where a theme could fail to be applied.
///
/// It asks for no repaint of its own, which is what keeps that from being a
/// loop: `Context::set_style_of` writes into the context's options and nothing
/// else.
pub(crate) fn apply(ctx: &egui::Context) {
    // Forced, rather than following whatever the desktop asks for. This window
    // is a viewer for pictures, and a picture is judged against the surround it
    // is judged against — a light editor lies about every exposure in the film.
    // It is the one thing about the look the user named outright.
    ctx.set_theme(egui::Theme::Dark);
    // Built from `egui`'s default rather than from whatever is installed, which
    // is what makes this idempotent: read the current style and adjust it and
    // sixty calls a second compound into sixty adjustments. Everything below
    // replaces a field outright.
    let mut style = egui::Style::default();
    text(&mut style);
    spacing(&mut style);
    style.visuals = visuals();
    // Written into *both* themes, so that nothing — a platform signalling a
    // preference mid-session, a future setting, an `egui` default — can leave
    // half the window drawn in a theme this module never described.
    let style: std::sync::Arc<egui::Style> = style.into();
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.set_style_of(theme, style.clone());
    }
}

/// The type scale.
///
/// Smaller than `egui`'s default across the board, because the window is a
/// dense one: four panels, a timeline and a preview on a 1280-wide window. The
/// monospace size is the one that matters most — every number in this app is a
/// frame count or a timecode, and numbers that do not line up in a column are
/// numbers you compare by reading rather than by looking.
fn text(style: &mut egui::Style) {
    use FontFamily::{Monospace, Proportional};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(15.0, Proportional)),
        (TextStyle::Body, FontId::new(13.0, Proportional)),
        (TextStyle::Button, FontId::new(12.5, Proportional)),
        (TextStyle::Small, FontId::new(10.5, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
    ]
    .into();
    // Every editable number in this window is a frame count, and a frame count
    // is read digit by digit. Proportional digits in a field that is being
    // dragged shift under the pointer as the value passes 99.
    style.drag_value_text_style = TextStyle::Monospace;
}

/// The gaps.
///
/// Tighter horizontally than vertically: rows in the inspector and the pool are
/// scanned down a column, and vertical air is what makes a column scannable,
/// while horizontal air only pushes the value away from the label it belongs to.
fn spacing(style: &mut egui::Style) {
    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(7.0, 5.0);
    spacing.button_padding = egui::vec2(8.0, 3.0);
    spacing.menu_margin = Margin::same(6);
    spacing.indent = 14.0;
    spacing.interact_size.y = 20.0;
    spacing.scroll.bar_width = 7.0;
    spacing.scroll.floating = false;
}

/// The colours, and the two structural decisions in them.
///
/// **A widget at rest has no fill and no outline.** `egui`'s default draws a
/// grey box around every button whether or not anything is happening, which on a
/// dark panel turns a row of controls into a row of boxes. Here a control is its
/// label until the pointer arrives, and then it lights.
///
/// **The only outlines are hairlines.** One pixel, in [`palette::RULE`] or
/// [`palette::EDGE`], never in a widget's own colour — so the accent stays
/// meaning "this is the thing you have selected" rather than "this is a button".
fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = palette::INK;
    // The same ground as a panel, not a lighter one. A dialog here floats over
    // the preview's near-black matte, so what separates it is its outline and
    // the dark halo under it — and filling it in [`palette::RAISED`] would make
    // every field inside it, which is also `RAISED`, disappear into it.
    visuals.window_fill = palette::INK;
    visuals.extreme_bg_color = palette::VOID;
    visuals.faint_bg_color = Color32::from_rgb(0x13, 0x18, 0x1E);
    visuals.code_bg_color = palette::RAISED;
    visuals.override_text_color = Some(palette::TEXT);
    visuals.weak_text_color = Some(palette::DIM);
    visuals.warn_fg_color = palette::WARM;
    visuals.error_fg_color = palette::ALERT;
    visuals.hyperlink_color = palette::ACCENT;

    visuals.window_corner_radius = ROUND;
    visuals.menu_corner_radius = ROUND;
    visuals.window_stroke = Stroke::new(1.0, palette::EDGE);
    visuals.window_shadow = shadow();
    visuals.popup_shadow = shadow();

    visuals.selection.bg_fill = palette::over(palette::ACCENT, palette::INK, 0.30);
    visuals.selection.stroke = Stroke::new(1.0, palette::ACCENT);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive = widget(Color32::TRANSPARENT, palette::RULE, palette::DIM);
    widgets.inactive = widget(palette::RAISED, Color32::TRANSPARENT, palette::TEXT);
    widgets.hovered = widget(palette::HOVER, palette::EDGE, palette::TEXT);
    widgets.active = widget(palette::ACTIVE, palette::ACCENT_DIM, Color32::WHITE);
    widgets.open = widget(palette::HOVER, palette::EDGE, palette::TEXT);
    // A control that grows when the pointer touches it is a control that nudges
    // everything beside it. In a panel of stacked rows that reads as the layout
    // being unstable.
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.expansion = 0.0;
    }
    visuals
}

/// One widget state, since all five are the same three questions.
fn widget(fill: Color32, outline: Color32, text: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, outline),
        fg_stroke: Stroke::new(1.0, text),
        corner_radius: ROUND,
        expansion: 0.0,
    }
}

/// What floats above the panels: a dialog, a menu, a tooltip.
///
/// Nearly opaque black and barely spread. On a near-black ground a soft wide
/// shadow is invisible; what actually separates a dialog from the panel behind
/// it is a *dark halo* tight against its edge, under the one-pixel outline.
fn shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(190),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applying twice is applying once. The theme is installed on every repaint,
    /// so anything that read the current style and adjusted it would compound
    /// sixty times a second.
    #[test]
    fn applying_the_theme_twice_leaves_the_same_style() {
        let ctx = egui::Context::default();
        let style = |ctx: &egui::Context| ctx.style_of(egui::Theme::Dark);
        apply(&ctx);
        let once = style(&ctx);
        apply(&ctx);
        let twice = style(&ctx);
        assert_eq!(once.visuals.panel_fill, twice.visuals.panel_fill);
        assert_eq!(once.text_styles, twice.text_styles);
        assert_eq!(once.spacing.item_spacing, twice.spacing.item_spacing);
    }

    /// The one thing about the theme a person would notice before anything
    /// else, and the one thing the user asked for by name.
    #[test]
    fn the_window_is_dark() {
        let visuals = visuals();
        assert!(visuals.dark_mode);
        assert!(visuals.panel_fill.r() < 32 && visuals.panel_fill.b() < 40);
    }

    /// A control at rest is its label. See [`visuals`] for why.
    #[test]
    fn nothing_is_outlined_until_it_is_touched() {
        let visuals = visuals();
        assert_eq!(
            visuals.widgets.inactive.bg_stroke.color,
            Color32::TRANSPARENT
        );
        assert_ne!(
            visuals.widgets.hovered.bg_stroke.color,
            Color32::TRANSPARENT
        );
    }
}
