//! What `scorsese check` says about a project's text before anything is drawn:
//! a face nothing ships, and characters no face can draw.

use std::path::Path;

use scorsese_compositor::text;
use scorsese_core::{AssetKind, FontChoice, Project};

use super::Painter;

/// A text asset naming a font this build does not ship.
///
/// A **problem** rather than a warning, unlike an unknown keyframe property:
/// a track nothing animates leaves a render that is merely missing a fade,
/// where a face nothing can find leaves no render at all. So it is worth
/// finding before an encode starts rather than partway through one, which is
/// the whole reason `scorsese check` exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFont {
    /// The text asset naming it.
    pub asset: String,
    /// The name as authored, quoted back so it can be searched for.
    pub named: String,
    /// Every name this build does answer to — the question being asked at the
    /// moment somebody reads this.
    pub available: String,
}

impl std::fmt::Display for UnknownFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "there is no font called `{}`. The ones scorsese ships are: {}",
            self.named, self.available
        )
    }
}

/// Every text asset in the project naming a shipped face that does not exist.
///
/// Only names: a `font` that is a **path** is a file on disk, and whether it is
/// there is `check`'s media pass to answer, alongside every other missing file.
pub fn unknown_fonts(project: &Project) -> Vec<UnknownFont> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Text)
        .filter_map(|asset| {
            let named = asset.text_style().font.name()?.to_owned();
            text::family(&named).is_none().then(|| UnknownFont {
                asset: asset.id.to_string(),
                named,
                available: text::names().collect::<Vec<_>>().join(", "),
            })
        })
        .collect()
}

/// A text asset saying something **no face scorsese has** can draw.
///
/// A warning and never a problem: the render succeeds, the frames are fine
/// everywhere else, and swapping the face or the character is the author's
/// call rather than this command's. What makes it worth saying at all is that
/// the alternative to saying it is finding out by eye — an unmapped character
/// is dropped **with its advance**, so the line closes up and looks like text
/// nobody wrote rather than text that failed.
///
/// The named face is not the whole question, since font fallback arrived: a
/// character it lacks and the emoji face draws reaches the frame, and objecting
/// to it would be a warning about something correct. What is left here is what
/// still vanishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredGlyphs {
    /// The text asset whose content it is.
    pub asset: String,
    /// The face as authored — a shipped name or a path — so the answer is
    /// "swap this" rather than "swap something".
    pub face: String,
    /// The characters it cannot draw, each named once, in the order they
    /// first appear.
    pub characters: Vec<char>,
}

impl std::fmt::Display for UncoveredGlyphs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The code point beside the character, because half of these are
        // invisible in a terminal and `U+2713` is the searchable half.
        let listed = self
            .characters
            .iter()
            .map(|character| format!("`{character}` (U+{:04X})", *character as u32))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "`{}` has no glyph for {listed}, and nor does any face scorsese \
             falls back to — they are dropped, not drawn",
            self.face
        )
    }
}

/// Every text asset saying something no face in its chain can draw.
///
/// Resolves each asset's face exactly as a render would, through the same
/// painter, so this can never disagree with what the frames do. A face that
/// will not open at all is not reported here — that is `unknown_fonts` above,
/// or the media pass, and reporting it twice would be two findings for one
/// fault.
pub fn uncovered_glyphs(project: &Project, project_root: &Path) -> Vec<UncoveredGlyphs> {
    let mut painter = Painter::default();
    let mut found = Vec::new();
    for asset in project.assets.iter().filter(|a| a.kind == AssetKind::Text) {
        if asset.text.is_none() {
            continue;
        }
        // With its figure written in, since a counter's digits and separators
        // are characters the face has to have too.
        let content = super::typing::written(asset);
        let style = asset.text_style();
        let Ok(font) = painter.font(&style, asset, project_root) else {
            continue;
        };
        let characters = font.uncovered(&content);
        if characters.is_empty() {
            continue;
        }
        found.push(UncoveredGlyphs {
            asset: asset.id.to_string(),
            face: face_named(&style.font),
            characters,
        });
    }
    found
}

/// The face as the document wrote it — a shipped name, or the path the
/// project carries. `FontChoice` is serialised as a plain string either way,
/// which is the same string an author would search for.
fn face_named(choice: &FontChoice) -> String {
    match choice {
        FontChoice::Named(name) => name.clone(),
        FontChoice::File(path) => path.to_string(),
    }
}
