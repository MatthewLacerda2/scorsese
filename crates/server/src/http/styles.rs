//! `GET /api/styles`: the menu a project starts from (#1016) — every
//! placement and every style in `scorsese_core::style`'s library, the same
//! closed lists the CLI and MCP offer.
//!
//! A style's preview is served data, web-only (#1013); until #1017 renders
//! them, every `preview` is `null` and the web app draws a neutral card.

use axum::Json;
use scorsese_core::style::{Platform, STYLES, Style};
use serde::Serialize;

use super::auth::Member;

/// The menu: placements in the order they are offered, styles likewise.
#[derive(Debug, Serialize)]
pub struct Menu {
    /// Every placement.
    pub platforms: Vec<PlatformView>,
    /// Every style.
    pub styles: Vec<StyleView>,
}

/// A placement as the dialog offers it.
#[derive(Debug, Serialize)]
pub struct PlatformView {
    /// What the API takes.
    pub id: &'static str,
    /// What a person is shown.
    pub name: &'static str,
    /// Whether it is a paid placement.
    pub ad: bool,
    /// The size a render for it is delivered at.
    pub width: u32,
    /// See `width`.
    pub height: u32,
}

/// A style as the dialog offers it.
#[derive(Debug, Serialize)]
pub struct StyleView {
    /// What the API takes.
    pub id: &'static str,
    /// What a person is shown.
    pub name: &'static str,
    /// One line, at most thirty words.
    pub description: &'static str,
    /// The placements it is made for, by id.
    pub platforms: Vec<&'static str>,
    /// Where its animated preview is served from — `null` until there is one.
    pub preview: Option<String>,
}

/// `GET /api/styles`: the menu. The same for every member; signed in only
/// because nothing else in `/api` answers a stranger.
pub async fn menu(_member: Member) -> Json<Menu> {
    Json(Menu {
        platforms: Platform::ALL.into_iter().map(platform).collect(),
        styles: STYLES.iter().map(style).collect(),
    })
}

fn platform(platform: Platform) -> PlatformView {
    let (width, height) = platform.size();
    PlatformView {
        id: platform.id(),
        name: platform.name(),
        ad: platform.is_ad(),
        width,
        height,
    }
}

fn style(style: &Style) -> StyleView {
    StyleView {
        id: style.id,
        name: style.name,
        description: style.description,
        platforms: style
            .platforms
            .iter()
            .map(|platform| platform.id())
            .collect(),
        preview: None,
    }
}
