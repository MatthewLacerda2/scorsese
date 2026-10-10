//! The brief a project starts with when it is started for a platform, a style,
//! or both (#1015): written into the project's script, because the script is
//! the project's brief by definition — never rendered, never parsed.
//!
//! One function every surface calls, so `scorsese new`, the MCP `project_new`
//! tool and the web app's new-project dialog start a project with the same
//! words. Nothing reads the brief back: once written it is the script, and the
//! assistant edits it like any other text — which is why changing the style
//! later is an edit, not a tool.

use std::fmt::Write as _;
use std::path::Path;

use super::{Platform, STYLES, Style, style, styles_for};
use crate::asset::Aspect;
use crate::project::{Project, SaveError};
use crate::{ProjectPath, write};

/// Where a started project's brief goes: the script's conventional name.
pub const SCRIPT_FILE: &str = "script.md";

/// What a project is being started for. Neither is required; with neither,
/// a project starts exactly as it always has, with no script at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Start {
    /// The placement the video is made for.
    pub platform: Option<Platform>,
    /// The kind of video it is.
    pub style: Option<&'static Style>,
}

/// Why a [`Start`] was refused. Each message names what would be accepted, so
/// whoever asked can pick again without looking anything up.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StartError {
    /// The style id names nothing in the library.
    #[error("no style `{0}`; one of {list}", list = ids(STYLES.iter()))]
    UnknownStyle(String),
    /// The style is not made for the placement asked for.
    #[error(
        "the style `{style}` is not made for {platform}; one of {list}",
        list = ids(styles_for(*platform))
    )]
    Unsuited {
        /// The style asked for.
        style: &'static str,
        /// The placement it does not suit.
        platform: Platform,
    },
}

impl Start {
    /// A start for `platform` and the style called `style_id`, refusing an id
    /// the library does not have and a style not made for that placement.
    ///
    /// A mismatch is refused rather than written down: the library already
    /// says which placements each style is made for, and the web's dialog only
    /// offers those, so a mismatch here is a typo more often than a plan.
    pub fn new(platform: Option<Platform>, style_id: Option<&str>) -> Result<Self, StartError> {
        let style = style_id
            .map(|id| style(id).ok_or_else(|| StartError::UnknownStyle(id.to_owned())))
            .transpose()?;
        if let (Some(platform), Some(style)) = (platform, style)
            && !style.suits(platform)
        {
            return Err(StartError::Unsuited {
                style: style.id,
                platform,
            });
        }
        Ok(Self { platform, style })
    }

    /// The brief for this start, or `None` when nothing was chosen.
    pub fn brief(&self) -> Option<String> {
        if self.platform.is_none() && self.style.is_none() {
            return None;
        }
        let mut text = String::from("# Brief\n\n");
        if let Some(platform) = self.platform {
            platform_section(&mut text, platform);
        }
        if let Some(style) = self.style {
            style_section(&mut text, style);
        }
        text.push_str(NEXT_STEP);
        Some(text)
    }

    /// Writes the brief into `project`'s script and points the document at
    /// it, returning where it went — or does nothing, and returns `None`,
    /// when nothing was chosen.
    ///
    /// For a project just created, which has no script yet. The file lands
    /// before the field is set, so a failed write never leaves the document
    /// claiming a script it does not have.
    pub fn write(
        &self,
        project_dir: &Path,
        project: &mut Project,
    ) -> Result<Option<ProjectPath>, SaveError> {
        let Some(brief) = self.brief() else {
            return Ok(None);
        };
        let script = ProjectPath::new(SCRIPT_FILE);
        let path = script.resolve(project_dir);
        write::atomically(&path, brief).map_err(|source| SaveError::Io { path, source })?;
        project.script = Some(script.clone());
        project.save(project_dir)?;
        Ok(Some(script))
    }
}

/// Which placement, and what it asks of the render and the cut.
fn platform_section(text: &mut String, platform: Platform) {
    let (width, height) = platform.size();
    let upright = platform.aspect() == Aspect::Tall;
    let shape = if upright { "upright" } else { "landscape" };
    let _ = write!(
        text,
        "## Platform: {name} (`{id}`)\n\n\
         Render it for this placement: `scorsese render --platform {id}`, or the \
         MCP `render` tool's `platform`. That delivers it {shape}, {width}x{height}; \
         the project itself never stores a platform.\n\n",
        name = platform.name(),
        id = platform.id(),
    );
    if upright {
        text.push_str(
            "Upright feeds lay their own buttons and captions over the frame's edges \
             and bottom: keep text and faces in the middle of the frame.\n\n",
        );
    }
    if platform.is_ad() {
        text.push_str(
            "This is a paid ad: it has seconds to earn attention and ends on one call \
             to action. The hook comes in the first two seconds, and the sound never \
             drops to silence.\n\n",
        );
    }
}

/// Which style, and its prompt whole: how this kind of video is made.
fn style_section(text: &mut String, style: &Style) {
    let _ = write!(
        text,
        "## Style: {name} (`{id}`)\n\n{prompt}\n\n",
        name = style.name,
        id = style.id,
        prompt = style.prompt,
    );
}

/// The script-first rule, which every started project carries.
const NEXT_STEP: &str = "\
## Next step: propose the script

Before importing, generating or placing anything, propose the script to the person \
and agree it with them. Scene by scene, each scene with:

- its narration — the words spoken, or none;
- what is on screen;
- the music or sound under it.

Write the agreed script into this file, keeping whatever above still holds, and \
build the video from it.
";

#[cfg(test)]
mod tests;

/// Some styles' ids, comma-separated.
fn ids<'a>(styles: impl Iterator<Item = &'a Style>) -> String {
    styles.map(|style| style.id).collect::<Vec<_>>().join(", ")
}
