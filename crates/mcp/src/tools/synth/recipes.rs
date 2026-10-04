//! Naming a recipe, and the rule that a name stays inside the project.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use scorsese_core::ProjectPath;
use serde::Deserialize;

use crate::tools::args::{self, Name, ProjectDir};

/// The `recipe` argument, described the same way in every tool that takes one.
pub(super) const RECIPE: &str = "Path to the recipe, relative to the project root — by \
                                 convention recipes/<name>.json. project_assets says which \
                                 recipe backs which asset.";

/// What `synth_check` and `synth_read` take: a project and a recipe inside it.
#[derive(Deserialize, JsonSchema)]
pub(super) struct RecipeArguments {
    pub(super) project: ProjectDir,
    #[schemars(description = RECIPE)]
    pub(super) recipe: Name,
}

impl args::Arguments for RecipeArguments {}

/// The recipe's real path, and how it is written down.
///
/// Checked against the project-relative rules before it is opened, so a recipe
/// argument is not a way to read or write outside the project — the same rule
/// the renderer and the synthesiser both hold.
pub(super) fn recipe_path(dir: &Path, recipe: &Name) -> Result<(PathBuf, String), String> {
    let relative = ProjectPath::new(recipe.as_str());
    relative
        .check()
        .map_err(|problem| format!("recipe path {problem}"))?;
    let file = relative.resolve(dir);
    Ok((file, relative.as_str().to_owned()))
}

/// A file, with the failure worded for a client.
pub(super) fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|error| format!("reading {}: {error}", file.display()))
}
