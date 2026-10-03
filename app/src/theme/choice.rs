//! Light or dark, and whether the person chose.
//!
//! The web's rule (`web/src/lib/theme.ts`, #582), restated: **an explicit
//! choice is remembered and wins; until there is one, the system decides**, and
//! is followed live. Following live costs nothing here — with no choice the
//! window asks egui for [`egui::ThemePreference::System`], and eframe passes
//! the platform's appearance through on every frame, so a desktop switching to
//! dark at sunset switches this window with it.
//!
//! ## Where the choice is kept
//!
//! **`app.json`, in the same folder as `settings.json`** —
//! `~/.config/scorsese/` on Linux, `~/Library/Application Support/scorsese/` on
//! macOS, `%APPDATA%\scorsese\` on Windows — found through the one resolver
//! `scorsese-providers` already has for that folder. Per machine, like the
//! spending ceiling, because how a window looks belongs to the person at it and
//! never travels with a project. Its own file rather than a key in
//! `settings.json`, because that file is the credentials resolver's: it refuses
//! keys it does not know, and every headless caller reads it — a window's
//! colours are not something a render or the server should ever parse.
//!
//! A file that is missing, unreadable or not understood reads as *not chosen*,
//! exactly as the web treats a `localStorage` that throws: the worst outcome is
//! following the system, never a window that will not open.

use std::path::PathBuf;

use egui::Theme;

/// The file's name, beside `settings.json`.
const FILE: &str = "app.json";

/// The person's choice, if they made one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Choice(Option<Theme>);

impl Choice {
    /// No choice: follow the system. What a fresh install, and every snapshot
    /// test, starts from.
    pub(crate) const SYSTEM: Self = Self(None);

    /// A theme chosen outright.
    pub(crate) const fn of(theme: Theme) -> Self {
        Self(Some(theme))
    }

    /// What this machine remembers.
    pub(crate) fn load() -> Self {
        let read = path().and_then(|path| std::fs::read_to_string(path).ok());
        Self(read.as_deref().and_then(parse))
    }

    /// Remembers this choice. A file that cannot be written means it lasts
    /// this session only, which is the web's answer to a private window too.
    pub(crate) fn save(self) {
        if let (Some(path), Some(theme)) = (path(), self.0) {
            if let Some(folder) = path.parent() {
                let _ = std::fs::create_dir_all(folder);
            }
            let _ = std::fs::write(path, render(theme));
        }
    }

    /// What to ask egui for: the choice, or the system's appearance.
    pub(crate) fn preference(self) -> egui::ThemePreference {
        self.0.map_or(egui::ThemePreference::System, Into::into)
    }

    /// The other theme from the one on screen, chosen outright — so the first
    /// press always visibly changes something, whatever the system said.
    pub(crate) fn toggled(showing: Theme) -> Self {
        Self::of(match showing {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        })
    }
}

/// Where the file goes, or `None` on a platform with no config folder.
fn path() -> Option<PathBuf> {
    let settings = scorsese_providers::credentials::settings_path().ok()?;
    Some(settings.parent()?.join(FILE))
}

/// The theme a file names, if it names one.
fn parse(text: &str) -> Option<Theme> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    match value.get("theme")?.as_str()? {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    }
}

/// The file for a theme. JSON, like `settings.json` beside it.
fn render(theme: Theme) -> String {
    let name = match theme {
        Theme::Dark => "dark",
        Theme::Light => "light",
    };
    format!("{}\n", serde_json::json!({ "theme": name }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What is written is what is read back.
    #[test]
    fn a_saved_choice_reads_back_as_itself() {
        for theme in [Theme::Dark, Theme::Light] {
            assert_eq!(parse(&render(theme)), Some(theme));
        }
    }

    /// Anything else is *not chosen* — the system decides, and the window
    /// still opens.
    #[test]
    fn a_file_nobody_understands_is_no_choice() {
        for text in ["", "{", "{}", r#"{"theme":"sepia"}"#, r#"{"theme":1}"#] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    /// No choice follows the system; a choice wins over it.
    #[test]
    fn a_choice_wins_and_no_choice_follows_the_system() {
        assert_eq!(Choice::SYSTEM.preference(), egui::ThemePreference::System);
        assert_eq!(
            Choice::of(Theme::Light).preference(),
            egui::ThemePreference::Light
        );
    }

    /// The toggle turns over what is showing, not what was stored: with the
    /// system dark and nothing chosen, the first press is light.
    #[test]
    fn the_toggle_turns_over_what_is_on_screen() {
        assert_eq!(Choice::toggled(Theme::Dark), Choice::of(Theme::Light));
        assert_eq!(Choice::toggled(Theme::Light), Choice::of(Theme::Dark));
    }
}
