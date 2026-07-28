//! Naming a recipe, and the rule that a name stays inside the project.

use std::path::{Path, PathBuf};

use scorsese_core::ProjectPath;
use serde_json::Value;

use super::super::{project_dir, project_property};

/// A schema of a project and a recipe inside it.
pub(super) fn recipe_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": { "project": project_property(), "recipe": recipe_property() },
        "required": ["project", "recipe"]
    })
}

/// The `recipe` property, spelled the same way in every tool that takes one.
pub(super) fn recipe_property() -> Value {
    serde_json::json!({
        "type": "string",
        "description": "Path to the recipe, relative to the project root — by \
                        convention recipes/<name>.json. project_assets says which \
                        recipe backs which asset."
    })
}

/// The project root, the recipe's real path, and how it is written down.
///
/// Checked against the project-relative rules before it is opened, so a recipe
/// argument is not a way to read or write outside the project — the same rule
/// the renderer and the synthesiser both hold.
pub(super) fn recipe_path(arguments: &Value) -> Result<(PathBuf, PathBuf, String), String> {
    let dir = project_dir(arguments)?;
    let relative = ProjectPath::new(text(arguments, "recipe")?);
    relative
        .check()
        .map_err(|problem| format!("recipe path {problem}"))?;
    let file = relative.resolve(&dir);
    Ok((dir, file, relative.as_str().to_owned()))
}

/// A required string argument.
pub(super) fn text<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("`{key}` is required"))
}

/// A file, with the failure worded for a client.
pub(super) fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|error| format!("reading {}: {error}", file.display()))
}
