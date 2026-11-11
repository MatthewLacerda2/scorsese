//! Reading every recipe a project carries, and saying what the set is made of.
//!
//! The one library call behind `scorsese synth survey`, so the CLI stays thin,
//! and the harmony of one song behind `synth_check`'s report and
//! `synth_bake`'s one line, so neither surface resolves an instrument its own
//! way. What a clash is, and why it is never a refusal, is
//! `scorsese_zimmer::survey`'s.
//! It used to be an MCP tool too (`synth_survey`) and left the tool list in
//! #785: it tunes the synthesiser's variety across songs, which is developing
//! `zimmer` rather than making a video.
//!
//! **It costs nothing.** No bake, no samples, no ffmpeg, no network — the
//! recipes are already on disk and this parses them. That is what makes it
//! reasonable to run on a whole project as often as anyone likes, and it is
//! also why it can only report what a document *says*: how a piece came out is
//! [`Baked`](super::Baked), and that costs a render.

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_zimmer::song::InlineOnly;
pub use scorsese_zimmer::survey::Harmony;
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
        let recipe = read_recipe(project, asset, project_root)?.recipe;
        if let Recipe::Song(song) = recipe {
            songs.push(SongSurvey::of(asset.id.as_str(), &song, &resolve));
        }
    }
    Ok(Survey { songs })
}

/// The harmony of `recipe`, or nothing for a one-shot.
///
/// `project_root` is where a song's named instruments are read from; without
/// one, a named instrument is read as pitched, since nothing says it is noise.
pub fn harmony(recipe: &Recipe, project_root: Option<&Path>) -> Option<Harmony> {
    let Recipe::Song(song) = recipe else {
        return None;
    };
    Some(match project_root {
        Some(root) => Harmony::of(song, &instruments(root)),
        None => Harmony::of(song, &InlineOnly),
    })
}

/// The harmony of the song asset `id`, read the way a bake reads it — a fit
/// to its clip included, which decides whether the piece loops.
///
/// Nothing for a one-shot, and for a recipe that cannot be read: a bake that
/// got this far has read it, and a report is not where it gets refused.
pub fn asset_harmony(project: &Project, project_root: &Path, id: &AssetId) -> Option<Harmony> {
    let asset = project.asset(id)?;
    let read = read_recipe(project, asset, project_root).ok()?;
    harmony(&read.recipe, Some(project_root))
}
