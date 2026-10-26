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
//! resolved, so a toggle reaches every
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
pub(crate) mod dialog;
mod fonts;
use fonts::{medium, semibold};
#[cfg(test)]
mod literals;
pub(crate) mod marks;
pub(crate) mod palette;
#[cfg(test)]
mod web_tokens;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Theme, Visuals};

use choice::Choice;
use palette::Palette;

/// The web's `--radius`, 0.625rem: ten pixels. The steps below are the ones
/// `web/src/index.css` derives from it.
const RADIUS: f32 = 10.0;

/// `--radius-sm`, 0.6 of the base: a clip, a lane, a tag, a chip.
pub(crate) const ROUND_SM: CornerRadius = CornerRadius::same((RADIUS * 0.6) as u8);
/// `--radius-md`, 0.8 of the base: a menu, and the picture's box.
pub(crate) const ROUND_MD: CornerRadius = CornerRadius::same((RADIUS * 0.8) as u8);
/// `--radius-lg`, the base itself: a control and a tile (`rounded-lg`).
pub(crate) const ROUND_LG: CornerRadius = CornerRadius::same(RADIUS as u8);
/// `--radius-xl`, 1.4 of the base: a dialog (`rounded-xl`).
pub(crate) const ROUND_XL: CornerRadius = CornerRadius::same((RADIUS * 1.4) as u8);

/// The text style of a heading: `text-xs font-semibold`.
pub(crate) fn semibold_small() -> TextStyle {
    TextStyle::Name("semibold-small".into())
}

/// The text style of a dialog's title: `text-base font-medium`.
pub(crate) fn medium_large() -> TextStyle {
    TextStyle::Name("medium-large".into())
}

/// Installs the whole look into `ctx`, light or dark as `choice` says.
///
/// Idempotent: each style is built from `egui`'s default rather than derived
/// from what is installed, so calling it on every repaint cannot compound, and
/// the faces are added only once (see [`fonts::install`]). It asks for no
/// repaint of its own: everything here writes the context's options and
/// nothing else.
pub(crate) fn apply(ctx: &egui::Context, choice: Choice) {
    fonts::install(ctx);
    let weights = fonts::weights_ready(ctx);
    if !weights {
        // Draw once more, in the weights.
        ctx.request_repaint();
    }
    ctx.set_theme(choice.preference());
    for (theme, palette) in [
        (Theme::Dark, &palette::DARK),
        (Theme::Light, &palette::LIGHT),
    ] {
        let mut style = egui::Style::default();
        text(&mut style, weights);
        spacing(&mut style);
        style.visuals = visuals(palette, theme);
        ctx.set_style_of(theme, style);
    }
}

/// The type scale: the web's. Panels are `text-sm` (14px), labels `text-xs`
/// (12px), buttons `text-sm font-medium`, and a heading `font-semibold`. Every
/// number here is a frame count or a timecode, so figures are monospaced:
/// digits that do not line up in a column are compared by reading rather than
/// by looking.
///
/// The weights are faces egui binds a frame after they are queued; until then
/// (`weights` false) the same sizes are drawn in the regular face.
fn text(style: &mut egui::Style, weights: bool) {
    use FontFamily::{Monospace, Proportional};
    let regular = |size| FontId::new(size, Proportional);
    let medium = |size| if weights { medium(size) } else { regular(size) };
    let semibold = |size| {
        if weights {
            semibold(size)
        } else {
            regular(size)
        }
    };
    style.text_styles = [
        (TextStyle::Heading, semibold(14.0)),
        (TextStyle::Body, regular(14.0)),
        (TextStyle::Button, medium(14.0)),
        (TextStyle::Small, regular(12.0)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
        (semibold_small(), semibold(12.0)),
        (medium_large(), medium(16.0)),
    ]
    .into();
    // A frame count being dragged is read digit by digit; proportional digits
    // shift under the pointer as the value passes 99.
    style.drag_value_text_style = TextStyle::Monospace;
}

/// The gaps: the web's. A control is 32px tall (`h-8`) with `px-2.5`, a panel
/// is `p-3`; a window that outgrows its inspector scrolls it.
fn spacing(style: &mut egui::Style) {
    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(8.0, 6.0);
    spacing.button_padding = egui::vec2(10.0, 4.0);
    spacing.menu_margin = Margin::same(4);
    spacing.window_margin = Margin::same(16);
    spacing.indent = 14.0;
    spacing.interact_size.y = 32.0;
    // Solid, not egui's floating default: that one draws its handle in the
    // text colour, which in the light theme is a black bar down the panel.
    // Solid takes the handle from the widget fill [`visuals`] sets.
    spacing.scroll = egui::style::ScrollStyle::solid();
    spacing.scroll.bar_width = 7.0;
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

    visuals.window_corner_radius = ROUND_XL;
    visuals.menu_corner_radius = ROUND_MD;
    // A dialog is the web's `ring-1 ring-foreground/10`, with no shadow.
    visuals.window_stroke = Stroke::new(1.0, palette::over(palette.foreground, palette.card, 0.1));
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.popup_shadow = shadow(theme);

    // A selected row is filled a step past hover, and selected text — a focused
    // field's outline among it — is drawn in the strongest neutral, the web's
    // `primary`. Grey, not a hue, for the reason in the module doc.
    visuals.selection.bg_fill = palette.pressed;
    // A focused field takes the web's `border-ring`; egui draws no halo.
    visuals.selection.stroke = Stroke::new(1.0, palette.ring);

    let widgets = &mut visuals.widgets;
    // Separators and a non-interactive frame are hairlines.
    widgets.noninteractive = widget(Color32::TRANSPARENT, palette.border, palette.foreground);
    widgets.inactive = widget(palette.field, palette.input, palette.foreground);
    widgets.hovered = widget(palette.hover, palette.input, palette.foreground);
    widgets.active = widget(palette.pressed, palette.ring, palette.foreground);
    widgets.open = widget(palette.hover, palette.input, palette.foreground);
    // A checkbox's box, a slider's rail and a scroll bar's handle are
    // `bg_fill`, not the button fill: at rest they need to show against the
    // panel, and the web draws a scroll bar's thumb in its border grey.
    widgets.inactive.bg_fill = palette.border;
    widgets.hovered.bg_fill = palette.ring;
    widgets.active.bg_fill = palette.ring;
    visuals
}

/// One widget state, since all five are the same three questions.
fn widget(fill: Color32, outline: Color32, text: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, outline),
        fg_stroke: Stroke::new(1.0, text),
        corner_radius: ROUND_LG,
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
        apply(&ctx, Choice::DEFAULT);
        let once = ctx.style_of(Theme::Light);
        apply(&ctx, Choice::DEFAULT);
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
        apply(&ctx, Choice::DEFAULT);
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

    /// The web's sizes: a control is 32px (`h-8`), a button's text is 14px
    /// medium, a heading is semibold.
    #[test]
    fn controls_and_type_are_the_webs() {
        let ctx = egui::Context::default();
        // The faces are bound a pass after they are queued.
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |ui| apply(ui.ctx(), Choice::DEFAULT));
        }
        let style = ctx.style_of(Theme::Light);
        assert_eq!(style.spacing.interact_size.y, 32.0);
        assert_eq!(style.spacing.button_padding.x, 10.0);
        assert_eq!(style.text_styles[&TextStyle::Body].size, 14.0);
        assert_eq!(style.text_styles[&TextStyle::Small].size, 12.0);
        assert_eq!(style.text_styles[&TextStyle::Button], medium(14.0));
        assert_eq!(style.text_styles[&TextStyle::Heading], semibold(14.0));
    }

    /// A dialog is outlined at `foreground/10` and casts no shadow.
    #[test]
    fn a_dialog_has_a_ring_and_no_shadow() {
        let visuals = visuals(&palette::LIGHT, Theme::Light);
        assert_eq!(visuals.window_shadow, egui::epaint::Shadow::NONE);
        assert_ne!(visuals.window_stroke.color, palette::LIGHT.border);
        assert_eq!(visuals.window_corner_radius, ROUND_XL);
    }
}
