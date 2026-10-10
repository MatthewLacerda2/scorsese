//! The menu a video starts from: where it is going ([`Platform`]) and what
//! kind of video it is ([`Style`]) (#1013, #1014).
//!
//! Both are closed lists, compiled in, so the CLI, the MCP server and the web
//! server offer exactly the same ones. A platform is a render preset — the
//! size a placement is watched at — and never enters `project.json`. A style
//! is a prompt: a long, fixed text telling whoever does the editing how this
//! kind of video is made, and what it needs from the person asking for it.
//!
//! The library is the menu; the how-to of a craft stays in its guide. A style
//! says what to make and in what order, and leaves the mechanics — a page's
//! contract, a recipe's format — to the guide it names.
//!
//! [`style`] finds one by id, [`styles_for`] lists the ones that suit a
//! platform, and [`STYLES`] is the whole library in the order it is shown.

mod library;
mod platform;
mod prompts;
#[cfg(test)]
mod tests;

pub use library::STYLES;
pub use platform::{Platform, UnknownPlatform};

/// One kind of video a person can start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// What a call or a stored setting names it by, in English snake_case.
    pub id: &'static str,
    /// What a person is shown, in pt-BR — the web app's language.
    pub name: &'static str,
    /// One line for the person choosing, at most thirty words, in pt-BR.
    pub description: &'static str,
    /// The placements it is made for, in [`Platform::ALL`]'s order.
    pub platforms: &'static [Platform],
    /// How this kind of video is made, for the assistant doing the editing:
    /// its structure, its pacing, and what to ask the person for.
    pub prompt: &'static str,
}

impl Style {
    /// Whether the style is made for `platform`.
    pub fn suits(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }
}

/// The style called `id`, if the library has one.
pub fn style(id: &str) -> Option<&'static Style> {
    STYLES.iter().find(|style| style.id == id)
}

/// Every style made for `platform`, in the library's order.
pub fn styles_for(platform: Platform) -> impl Iterator<Item = &'static Style> {
    STYLES.iter().filter(move |style| style.suits(platform))
}
