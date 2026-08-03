//! Reading every recipe a project carries, and saying what the set is made of.
//!
//! The one library call behind `scorsese synth survey` and `synth_survey`, so
//! the CLI and the MCP server are both thin: whatever either of them shows, the
//! other shows too, because there is one place that decides it.
//!
//! **It costs nothing.** No bake, no samples, no ffmpeg, no network — the
//! recipes are already on disk and this parses them. That is what makes it
//! reasonable to run on a whole project as often as anyone likes, and it is
//! also why it can only report what a document *says*: how a piece came out is
//! [`Baked`](super::Baked), and that costs a render.

use std::path::Path;

use scorsese_core::Project;
use scorsese_zimmer::survey::{SongSurvey, Survey};

use super::error::SynthesisError;
use super::recipe::Recipe;
use super::{instruments, read_recipe};

/// Surveys every song recipe the project's `synth_audio` assets point at.
///
/// One-shot patches are left out rather than reported with empty columns: a
/// survey is about the pieces of music a project carries, and an effect has no
/// tempo, no arrangement and no mix to be the loudest thing in.
///
/// # Errors
///
/// When a recipe cannot be read or does not parse. A survey that quietly
/// skipped a broken document would under-count the set and say so in no way at
/// all, which is worse than the failure — and the same read that bakes it is
/// the read that fails here, so a project that surveys is one that bakes.
pub fn survey(project: &Project, project_root: &Path) -> Result<Survey, SynthesisError> {
    let resolve = instruments(project_root);
    let mut songs = Vec::new();
    for asset in project.assets.iter().filter(|it| it.kind.is_synthesized()) {
        let (recipe, _, _) = read_recipe(asset, project_root)?;
        if let Recipe::Song(song) = recipe {
            songs.push(SongSurvey::of(asset.id.as_str(), &song, &resolve));
        }
    }
    Ok(Survey { songs })
}
