//! The look: one module that decides every colour, face, radius and gap.
//!
//! ## What it is, and where it comes from
//!
//! **The web app's look** (#643). The web editor was laid out after this window
//! and came out cleaner, so the reference now runs the other way: a person
//! moving between the two should feel they are in one product. So this is the
//! web's visual language restated for egui —
//!
//! - **its tokens**: shadcn's neutral palette from `web/src/index.css`, light
//!   and dark, converted once into [`palette`] with a note on every value;
//! - **its face**: Geist, with Geist Mono for figures ([`fonts`]);
//! - **its radius scale**: `--radius` and the steps the web derives from it;
//! - **its controls**: shadcn's *outline* variant — a hairline and a faint fill
//!   at rest, a step lighter or darker under the pointer — which is what the
//!   web's inputs and selects look like, so a field and a button agree;
//! - **its headings**: small capitals, lightly tracked, in the secondary text
//!   colour (`text-xs uppercase tracking-wide text-muted-foreground`).
//!
//! The chrome is grey on purpose. Colour is kept for what a thing *is*: the
//! timeline's clips are coloured by kind exactly as the web's are, which is the
//! one thing of the previous look worth keeping — Filmora 9 colours its clips
//! too, and `CLAUDE.md` names it as the timeline's reference for taste. What
//! this replaced was a film-interface chrome (near-black blue-grey grounds, a
//! cyan accent, marks that framed rather than boxed) that only this window had.
//!
//! ## Light and dark
//!
//! Both, chosen at runtime by the web's rule — see [`choice`]. Every colour a
//! panel draws is read from [`palette::of`], which follows the theme egui has
//! resolved, so a toggle or a change in the system's appearance reaches every
//! panel on the next frame. The one thing that does not turn over is the black
//! a picture sits on ([`palette::MATTE`]): a picture is judged against its
//! surround, and a white one would lie about every exposure.
//!
//! ## Where it is applied
//!
//! From [`Scorsese::draw`](crate::Scorsese::draw), on every repaint, and **not**
//! from `main.rs`. The snapshot harness calls `draw` with no event loop and no
//! window, so a theme installed at startup would be a theme the reference images
//! never see — and the pictures in `app/tests/snapshots/` would be of a window
//! nobody uses.

pub(crate) mod choice;
mod fonts;
pub(crate) mod marks;
pub(crate) mod palette;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Theme, Visuals};

use choice::Choice;
use palette::Palette;

/// The web's `--radius`, 0.625rem: ten pixels. The steps below are the ones
/// `web/src/index.css` derives from it.
const RADIUS: f32 = 10.0;

/// `--radius-sm`, 0.6 of the base: a clip, a lane, a tag, a chip.
pub(crate) const ROUND_SM: CornerRadius = CornerRadius::same((RADIUS * 0.6) as u8);
/// `--radius-md`, 0.8 of the base: a button, a field, a menu — what the web's
/// compact buttons use.
pub(crate) const ROUND_MD: CornerRadius = CornerRadius::same((RADIUS * 0.8) as u8);
/// `--radius-lg`, the base itself: a dialog.
pub(crate) const ROUND_LG: CornerRadius = CornerRadius::same(RADIUS as u8);

/// Installs the whole look into `ctx`, light or dark as `choice` and the
/// system decide.
///
/// Idempotent: each style is built from `egui`'s default rather than derived
/// from what is installed, so calling it on every repaint cannot compound, and
/// the faces are added only once (see [`fonts::install`]). It asks for no
/// repaint of its own: everything here writes the context's options and
/// nothing else.
pub(crate) fn apply(ctx: &egui::Context, choice: Choice) {
    fonts::install(ctx);
    ctx.set_theme(choice.preference());
    for (theme, palette) in [
        (Theme::Dark, &palette::DARK),
        (Theme::Light, &palette::LIGHT),
    ] {
        let mut style = egui::Style::default();
        text(&mut style);
        spacing(&mut style);
        style.visuals = visuals(palette, theme);
        ctx.set_style_of(theme, style);
    }
}

/// The type scale.
///
/// The web's sizes, a step down where this window is denser: its panels are
/// `text-sm` (14px) and its labels `text-xs` (12px), and this window puts four
/// panels and a timeline in 1280 pixels. Every number here is a frame count or
/// a timecode, so figures are monospaced: digits that do not line up in a
/// column are compared by reading rather than by looking.
fn text(style: &mut egui::Style) {
    use FontFamily::{Monospace, Proportional};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, Proportional)),
        (TextStyle::Body, FontId::new(13.0, Proportional)),
        (TextStyle::Button, FontId::new(13.0, Proportional)),
        (TextStyle::Small, FontId::new(11.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
    ]
    .into();
    // A frame count being dragged is read digit by digit; proportional digits
    // shift under the pointer as the value passes 99.
    style.drag_value_text_style = TextStyle::Monospace;
}

/// The gaps: the web's density, which is a little airier than this window
/// was — controls 24px tall (the web's `xs` button, `h-6`), padded `px-2`.
fn spacing(style: &mut egui::Style) {
    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(8.0, 6.0);
    spacing.button_padding = egui::vec2(8.0, 3.0);
    spacing.menu_margin = Margin::same(4);
    spacing.window_margin = Margin::same(16);
    spacing.indent = 14.0;
    spacing.interact_size.y = 24.0;
    spacing.scroll.bar_width = 7.0;
    spacing.scroll.floating = false;
}

/// The colours, as the web draws a panel, a control and a selection.
fn visuals(palette: &Palette, theme: Theme) -> Visuals {
    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.card;
    visuals.extreme_bg_color = palette.background;
    visuals.text_edit_bg_color = Some(palette.field);
    visuals.faint_bg_color = palette.muted;
    visuals.code_bg_color = palette.muted;
    visuals.override_text_color = Some(palette.foreground);
    visuals.weak_text_color = Some(palette.muted_foreground);
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.destructive;
    visuals.hyperlink_color = palette.foreground;

    visuals.window_corner_radius = ROUND_LG;
    visuals.menu_corner_radius = ROUND_MD;
    visuals.window_stroke = Stroke::new(1.0, palette.border);
    visuals.window_shadow = shadow(theme);
    visuals.popup_shadow = shadow(theme);

    // A selected row is filled a step past hover, and selected text — a focused
    // field's outline among it — is drawn in the strongest neutral, the web's
    // `primary`. Grey, not a hue, for the reason in the module doc.
    visuals.selection.bg_fill = palette.pressed;
    visuals.selection.stroke = Stroke::new(1.0, palette.primary);

    let widgets = &mut visuals.widgets;
    // Separators and a non-interactive frame are hairlines.
    widgets.noninteractive = widget(Color32::TRANSPARENT, palette.border, palette.foreground);
    widgets.inactive = widget(palette.field, palette.input, palette.foreground);
    widgets.hovered = widget(palette.hover, palette.input, palette.foreground);
    widgets.active = widget(palette.pressed, palette.ring, palette.foreground);
    widgets.open = widget(palette.hover, palette.input, palette.foreground);
    // A checkbox's box and a slider's rail are `bg_fill`, not the button fill,
    // and at rest they need to show against the panel.
    widgets.inactive.bg_fill = palette.muted;
    visuals
}

/// One widget state, since all five are the same three questions.
fn widget(fill: Color32, outline: Color32, text: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, outline),
        fg_stroke: Stroke::new(1.0, text),
        corner_radius: ROUND_MD,
        // A control that grows under the pointer nudges everything beside it.
        expansion: 0.0,
    }
}

/// What floats: a dialog, a menu, a tooltip. The web's `shadow-lg`, soft and
/// low, and stronger in the dark where a soft shadow on a dark ground is
/// otherwise invisible.
fn shadow(theme: Theme) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(match theme {
            Theme::Dark => 160,
            Theme::Light => 40,
        }),
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
        apply(&ctx, Choice::SYSTEM);
        let once = ctx.style_of(Theme::Light);
        apply(&ctx, Choice::SYSTEM);
        let twice = ctx.style_of(Theme::Light);
        assert_eq!(once.visuals.panel_fill, twice.visuals.panel_fill);
        assert_eq!(once.text_styles, twice.text_styles);
        assert_eq!(once.spacing.item_spacing, twice.spacing.item_spacing);
    }

    /// Each theme gets its own palette — the light style is not the dark one
    /// with a flag flipped.
    #[test]
    fn each_theme_is_drawn_in_its_own_palette() {
        let ctx = egui::Context::default();
        apply(&ctx, Choice::SYSTEM);
        let dark = ctx.style_of(Theme::Dark);
        let light = ctx.style_of(Theme::Light);
        assert!(dark.visuals.dark_mode && !light.visuals.dark_mode);
        assert_eq!(dark.visuals.panel_fill, palette::DARK.background);
        assert_eq!(light.visuals.panel_fill, palette::LIGHT.background);
    }

    /// A choice reaches the context, and is what the panels then read.
    #[test]
    fn a_choice_is_the_theme_in_force() {
        let ctx = egui::Context::default();
        apply(&ctx, Choice::of(Theme::Light));
        assert_eq!(ctx.theme(), Theme::Light);
        apply(&ctx, Choice::of(Theme::Dark));
        assert_eq!(ctx.theme(), Theme::Dark);
    }

    /// A control at rest is outlined, as the web's inputs and outline buttons
    /// are, so a field reads as a field before the pointer finds it.
    #[test]
    fn a_control_at_rest_has_its_outline() {
        for (palette, theme) in [(palette::DARK, Theme::Dark), (palette::LIGHT, Theme::Light)] {
            let visuals = visuals(&palette, theme);
            assert_eq!(visuals.widgets.inactive.bg_stroke.color, palette.input);
            assert_ne!(
                visuals.widgets.hovered.weak_bg_fill,
                visuals.widgets.inactive.weak_bg_fill
            );
        }
    }
}
