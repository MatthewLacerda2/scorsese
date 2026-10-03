//! The window's typeface: Geist, the web app's, and Geist Mono beside it.
//!
//! Provenance and licence are in `app/assets/README.md`.

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{FontData, FontFamily};

/// Geist Regular, for everything proportional.
const SANS: &[u8] = include_bytes!("../../assets/fonts/Geist-Regular.ttf");
/// Geist Mono Regular, for every figure read digit by digit.
const MONO: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Regular.ttf");

/// Puts both faces in front of egui's own.
///
/// In front, not instead: egui's fonts stay behind as the fallback, so a glyph
/// Geist does not draw — the transport's `⏮` — is still drawn rather than
/// boxed. Safe to call every frame: egui adds a face only when no face of that
/// name is installed yet, so after the first frame this is a lookup.
pub(super) fn install(ctx: &egui::Context) {
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
