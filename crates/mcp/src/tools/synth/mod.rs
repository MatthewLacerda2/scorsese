//! Sound from a recipe: making one, editing it, and baking it.
//!
//! The loop these exist for is **write, bake, listen, adjust**. Two of them —
//! `synth_read` and `synth_write` — have no command-line counterpart, and that
//! is the point: over the CLI you would edit the file with an editor, and an
//! assistant that has to round-trip through the filesystem to change a note is
//! an assistant doing bookkeeping instead of composing.
//!
//! `synth_set` is the same argument one turn further in. Writing a score and
//! **tuning** one are different acts, and only the first is a whole document:
//! the adjust in write-bake-listen-adjust is one number at a time, several
//! times over, and paying for the entire piece to move a track's `gain` is
//! bookkeeping again in a new place.

mod recipes;
mod set;
mod survey;

pub(super) use set::Set;
pub(super) use survey::Survey;

use scorsese_core::ProjectPath;
use scorsese_providers::synth::{self, Baked, Starter};
use serde_json::Value;

use super::inspect::load;
use super::{Reply, Tool, project_dir, project_property};
use recipes::{read, recipe_path, recipe_property, recipe_schema, text};

/// Start a recipe.
pub(super) struct New;

impl Tool for New {
    fn name(&self) -> &'static str {
        "synth_new"
    }

    fn description(&self) -> &'static str {
        "Start a new sound: writes a starter recipe into recipes/ and adds the \
         synth_audio asset that points at it. The starter makes a sound as \
         written, so bake it and listen before changing anything. Costs nothing \
         — synthesis needs no key, no network and no money."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "name": {
                    "type": "string",
                    "description": "What to call it. Becomes the asset id and the \
                                    recipe's file name, suffixed if that is taken."
                },
                "kind": {
                    "type": "string",
                    "enum": ["patch", "song"],
                    "description": "`patch` for one instrument playing one note — an \
                                    effect. `song` for an arrangement — a score. \
                                    Default `patch`."
                }
            },
            "required": ["project", "name"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let name = text(arguments, "name")?;
        let starter = match arguments.get("kind").and_then(Value::as_str) {
            Some("song") => Starter::Song,
            None | Some("patch") => Starter::Patch,
            Some(other) => return Err(format!("`kind` is `patch` or `song`, not `{other}`")),
        };

        let id =
            synth::create(&mut project, &dir, name, starter).map_err(|error| format!("{error}"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        let recipe = project
            .asset(&id)
            .and_then(|asset| asset.recipe.as_ref())
            .map(ProjectPath::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(format!(
            "{id} — synth_audio, sketch\n{recipe}\nEdit it with synth_write, then \
             synth_bake to hear it."
        )
        .into())
    }
}

/// Render the recipes that are not already baked.
pub(super) struct Bake;

impl Tool for Bake {
    fn name(&self) -> &'static str {
        "synth_bake"
    }

    fn description(&self) -> &'static str {
        "Render every synth_audio recipe whose sound is not already on disk, \
         into generated/. Safe to call repeatedly and free every time: a recipe \
         that has not changed is a cache hit that renders nothing, and one that \
         has changed is redone without anyone having to mark it stale. Says how \
         each one came out: level and spectral balance for the whole file, then \
         a row per section of the arrangement, then a row per track of a song \
         saying which instrument is taking up the room. A signal, never a gate \
         — nothing about a level can fail a bake."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "asset": {
                    "type": "string",
                    "description": "Bake only this asset. Omit and every synth_audio \
                                    asset is considered."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;

        let baked = match arguments.get("asset").and_then(Value::as_str) {
            Some(id) => {
                let id = scorsese_core::AssetId::new(id);
                let one = synth::bake_asset(&mut project, &dir, &id)
                    .map_err(|error| format!("{error}"))?;
                vec![(id, one)]
            }
            None => synth::bake_pending(&mut project, &dir).map_err(|error| format!("{error}"))?,
        };
        if baked.is_empty() {
            return Ok("no synth_audio assets — synth_new starts one".into());
        }
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let fresh = baked.iter().filter(|(_, it)| it.is_fresh()).count();
        let lines: Vec<String> = baked
            .iter()
            .map(|(id, outcome)| match outcome {
                Baked::Rendered {
                    path,
                    bytes,
                    profile,
                    tracks,
                } => {
                    // An assistant that just rewrote a score should be able to
                    // read back how it came out without asking a human to
                    // listen — and the whole table, not one number, because the
                    // question it is usually answering is *where* in the piece
                    // the change landed.
                    let mut said = format!(
                        "{id} — baked, {} KB, {path}, {}",
                        bytes / 1024,
                        scorsese_render::say::summary(profile)
                    );
                    for row in scorsese_render::say::sections(profile) {
                        said.push_str(&format!("\n  {row}"));
                    }
                    // And *which layer* the energy is in, which is the other
                    // half of the same question: a client that cannot hear can
                    // be told a mix is muddy, and can do nothing with that
                    // until it is told which of five instruments is the mud.
                    for row in scorsese_render::say::layers(tracks) {
                        said.push_str(&format!("\n  {row}"));
                    }
                    said
                }
                Baked::Cached { path } => format!("{id} — already baked, {path}"),
            })
            .collect();
        Ok(format!(
            "{}\n{fresh} rendered, {} cached, $0.00",
            lines.join("\n"),
            baked.len() - fresh
        )
        .into())
    }
}

/// Parse a recipe without rendering it.
pub(super) struct Check;

impl Tool for Check {
    fn name(&self) -> &'static str {
        "synth_check"
    }

    fn description(&self) -> &'static str {
        "Parse a recipe and say what it is, without rendering it. Milliseconds \
         rather than the seconds a bake takes, so this is the fast way to find \
         out a document is malformed before spending a render on it."
    }

    fn schema(&self) -> Value {
        recipe_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let (_, file, relative) = recipe_path(arguments)?;
        let json = read(&file)?;
        match synth::check(&json) {
            Ok(parsed) => {
                Ok(format!("{relative}: a {} recipe, and it parses", parsed.kind()).into())
            }
            Err(problem) => Err(format!("{relative}: {problem}")),
        }
    }
}

/// The recipe as it stands.
pub(super) struct Read;

impl Tool for Read {
    fn name(&self) -> &'static str {
        "synth_read"
    }

    fn description(&self) -> &'static str {
        "Read a recipe file as it is on disk. Pair with synth_write to change a \
         sound: read it, change it, write it back, bake. The recipe format is \
         documented in docs/recipes.md."
    }

    fn schema(&self) -> Value {
        recipe_schema()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let (_, file, _) = recipe_path(arguments)?;
        read(&file).map(Into::into)
    }
}

/// Replace the recipe.
pub(super) struct Write;

impl Tool for Write {
    fn name(&self) -> &'static str {
        "synth_write"
    }

    fn description(&self) -> &'static str {
        "Replace a recipe file with the document given. Parsed before it is \
         written — a document that is not a recipe is refused and the file on \
         disk is left as it was. Writing a recipe makes its asset stale by \
         arithmetic: the bake is named for the recipe's hash, so the next \
         synth_bake redoes it and nothing has to be marked."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "recipe": recipe_property(),
                "document": {
                    "type": "string",
                    "description": "The complete recipe JSON to write. Not a patch — \
                                    whatever is here replaces the file."
                }
            },
            "required": ["project", "recipe", "document"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let (_, file, relative) = recipe_path(arguments)?;
        let document = text(arguments, "document")?;
        // Parsed before it is written, for the same reason `project_write`
        // validates: a recipe that is not a recipe makes every later bake fail
        // with a message about a file nobody remembers editing.
        let parsed = synth::check(document).map_err(|problem| {
            format!("refused, nothing written — {relative} would not parse: {problem}")
        })?;
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("creating {}: {error}", parent.display()))?;
        }
        scorsese_core::write::atomically(&file, document)
            .map_err(|error| format!("writing {relative}: {error}"))?;
        Ok(format!(
            "{relative} written — a {} recipe. Its asset is stale now; \
             synth_bake redoes it.",
            parsed.kind()
        )
        .into())
    }
}
