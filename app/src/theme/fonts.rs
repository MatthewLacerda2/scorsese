//! The window's typeface: Geist, the web app's, and Geist Mono beside it.
//!
//! Provenance and licence are in `app/assets/README.md`.
//!
//! egui draws a font at one weight, so the web's medium and semibold are their
//! own faces, each a family of its own ([`medium`], [`semibold`]).

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{FontData, FontDefinitions, FontFamily, FontId};

/// Geist Regular, for everything proportional.
const SANS: &[u8] = include_bytes!("../../assets/fonts/Geist-Regular.ttf");
/// Geist Medium: buttons, track names, dialog titles.
const MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Geist-Medium.ttf");
/// Geist SemiBold: the project's name, headings.
const SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Geist-SemiBold.ttf");
/// Geist Mono Regular, for every figure read digit by digit.
const MONO: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Regular.ttf");

/// Puts both faces in front of egui's own.
///
/// In front, not instead: egui's fonts stay behind as the fallback, so a glyph
/// Geist does not draw — the transport's `⏮` — is still drawn rather than
/// boxed. Safe to call every frame: egui adds a face only when no face of that
/// name is installed yet, so after the first frame this is a lookup.
pub(super) fn install(ctx: &egui::Context) {
    for (name, data) in [("Geist Medium", MEDIUM), ("Geist SemiBold", SEMIBOLD)] {
        let family = FontFamily::Name(name.into());
        ctx.add_font(FontInsert::new(
            name,
            FontData::from_static(data),
            vec![InsertFontFamily {
                family: family.clone(),
                priority: FontPriority::Highest,
            }],
        ));
        // A face of its own has no fallback of its own: give it egui's, so a
        // symbol in a medium button is drawn rather than boxed.
        let defaults = FontDefinitions::default();
        for fallback in &defaults.families[&FontFamily::Proportional] {
            ctx.add_font(FontInsert::new(
                &format!("{name} fallback {fallback}"),
                (*defaults.font_data[fallback]).clone(),
                vec![InsertFontFamily {
                    family: family.clone(),
                    priority: FontPriority::Lowest,
                }],
            ));
        }
    }
    for (name, data, family) in [
        ("Geist", SANS, FontFamily::Proportional),
        ("Geist Mono", MONO, FontFamily::Monospace),
    ] {
        ctx.add_font(FontInsert::new(
            name,
            FontData::from_static(data),
            vec![InsertFontFamily {
                family,
                priority: FontPriority::Highest,
            }],
        ));
    }
}

/// Whether the weights are bound, so a text style may name them. They are queued
/// with the first call and bound at the start of the next pass; a style naming
/// one earlier would panic where text is laid out.
pub(super) fn weights_ready(ctx: &egui::Context) -> bool {
    let id = egui::Id::new("scorsese weights queued");
    let now = ctx.cumulative_pass_nr();
    let queued = ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id, || now));
    now > queued
}

/// The web's `font-medium` at `size`.
pub(super) fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Geist Medium".into()))
}

/// The web's `font-semibold` at `size`.
pub(super) fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Geist SemiBold".into()))
}
