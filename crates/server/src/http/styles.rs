//! `GET /api/styles`: the menu a project starts from (#1016) — every
//! placement and every style in `scorsese_core::style`'s library, the same
//! closed lists the CLI and MCP offer.
//!
//! A style's preview is served data, web-only (#1013); until #1017 renders
//! them, every `preview` is `null` and the web app draws a neutral card.
//!
//! The names and descriptions here are the library's pt-BR; the web app shows
//! each in the reader's language from its own catalogues (#1051), keyed by id.
//! `web/src/start/menu.json` is this menu as served, and the test below holds
//! it to the library, so the web gate can hold its catalogues to the menu
//! without a running server. `make style-menu` rewrites it.

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
    Json(current())
}

/// The menu the library makes.
fn current() -> Menu {
    Menu {
        platforms: Platform::ALL.into_iter().map(platform).collect(),
        styles: STYLES.iter().map(style).collect(),
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use super::current;

    /// The web app's copy, found from this crate rather than from the working
    /// directory.
    fn copy() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/src/start/menu.json")
    }

    /// Compared as JSON values, not bytes: the web gate formats the file its
    /// own way, and only what it says has to match.
    #[test]
    fn the_web_apps_copy_of_the_menu_is_the_menu() {
        let served = serde_json::to_value(current()).expect("the menu serialises");
        let path = copy();
        let on_disk: Option<Value> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        if on_disk.as_ref() == Some(&served) {
            return;
        }
        if std::env::var_os("UPDATE_STYLE_MENU").is_some() {
            let text = serde_json::to_string_pretty(&served).expect("the menu serialises");
            std::fs::write(&path, text + "\n").expect("rewriting web/src/start/menu.json");
            return;
        }
        panic!(
            "web/src/start/menu.json is not the menu `scorsese_core::style` makes.\n\
             Run `make style-menu`, commit what it writes, and give any new platform \
             or style its words in web/src/i18n/*/menu.ts."
        );
    }
}
