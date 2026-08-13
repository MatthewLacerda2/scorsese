//! Editing the brief a generated asset is to be made from.
//!
//! One field and one state, written together, because doing only the first is
//! the mistake this tool exists to make impossible: an edited brief on an asset
//! still marked `generated` reads as up to date, so the cut keeps showing the
//! previous take and nothing anywhere says otherwise.
//!
//! **The two kinds of brief stay two.** A `prompt` is a sentence handed to a
//! provider; a `recipe` is a document the project carries. Each argument is
//! accepted by exactly the kinds `AssetKind::is_prompted` and `is_synthesized`
//! name, and refused by the others — the same split `Project::validate` holds
//! the document to, voiced here before the document is written rather than
//! after.

use scorsese_core::{Asset, AssetId, GenerationState, Project, ProjectPath};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Change a generated asset's brief, and its state with it.
pub(crate) struct Rebrief;

impl Tool for Rebrief {
    fn name(&self) -> &'static str {
        "rebrief"
    }

    fn description(&self) -> &'static str {
        "Change what a generated asset is to be made from, and mark it stale in \
         the same write. A generated asset runs sketch → queued → generated and \
         back to stale the moment its brief is edited — stale is what makes the \
         next generate redo it and what makes the cut show a slug card rather \
         than the previous take meanwhile, and doing it by hand is the half of \
         the edit that gets forgotten. Pass `prompt` for a generated_video or \
         generated_audio asset, whose brief is a sentence a provider is paid to \
         read, or `recipe` for a synth_audio asset, whose brief is a document in \
         recipes/ — one of the two, never both, and never the one the asset's \
         kind does not take. To change what is *inside* a recipe use synth_write \
         or synth_set; this repoints an asset at a different recipe file. A \
         brief nobody has generated yet stays sketch, because there is nothing \
         stale about it, and the reply always says which state the asset is in \
         now. The whole document is validated before anything is written, so a \
         refusal changes nothing. Nothing is generated here — call generate once \
         the brief reads right."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "asset": {
                    "type": "string",
                    "description": "Id of the generated asset whose brief to change — \
                                    the id in the assets table, never a path."
                },
                "prompt": {
                    "type": "string",
                    "description": "The new prompt, for a generated_video or \
                                    generated_audio asset. Replaces the old sentence \
                                    whole; this is not a patch. Refused on a \
                                    synth_audio asset, which has no prompt."
                },
                "recipe": {
                    "type": "string",
                    "description": "Project-relative path of the recipe a synth_audio \
                                    asset is to be baked from, by convention under \
                                    recipes/. The file has to exist. Refused on a \
                                    prompted asset, which has no recipe."
                }
            },
            "required": ["project", "asset"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let id = AssetId::new(required(arguments, "asset")?);
        let brief = Brief::asked(arguments)?;

        let Some(said) = edit(&mut project, &dir, &id, &brief)? else {
            return Ok(unchanged(&brief, &id));
        };
        // Validated before the save and not after: the refusal has to arrive
        // while the file on disk is still the old one, which is what makes
        // "nothing was changed" a fact rather than a hope.
        project
            .validate()
            .map_err(|problems| format!("refused, nothing written:\n{problems}"))?;
        project
            .save(&dir)
            .map_err(|error| format!("refused, nothing written: {error}"))?;
        Ok(said.into())
    }
}

/// A new brief, in whichever of its two forms was asked for.
enum Brief {
    /// A sentence, for the kinds a provider is paid to read one.
    Prompt(String),
    /// A document in the project, for the kinds synthesised from one.
    Recipe(ProjectPath),
}

impl Brief {
    /// Exactly one of the two, or a refusal saying which.
    ///
    /// Both at once is refused rather than resolved by kind: an asset carries
    /// exactly the brief its kind takes, so a call naming the other one has
    /// misunderstood the asset and guessing which half was meant would write
    /// the half nobody checked.
    fn asked(arguments: &Value) -> Result<Self, String> {
        let prompt = optional(arguments, "prompt");
        let recipe = optional(arguments, "recipe");
        match (prompt, recipe) {
            (Some(_), Some(_)) => Err("`prompt` and `recipe` are the two kinds of brief \
                                       and no asset takes both — pass the one this \
                                       asset's kind carries"
                .to_owned()),
            (Some(prompt), None) => Ok(Self::Prompt(prompt.to_owned())),
            (None, Some(recipe)) => Ok(Self::Recipe(ProjectPath::new(recipe))),
            (None, None) => Err("nothing to change: pass `prompt` for a generated_video \
                                 or generated_audio asset, or `recipe` for a synth_audio \
                                 one"
            .to_owned()),
        }
    }

    /// What this brief is called, in the words the document uses.
    fn field(&self) -> &'static str {
        match self {
            Self::Prompt(_) => "prompt",
            Self::Recipe(_) => "recipe",
        }
    }
}

/// Applies the brief to the asset, or says why it will not.
///
/// `Ok(None)` means the asset already reads exactly that: nothing is written
/// and — the point of the branch — a `generated` asset is not marked stale for
/// an edit that changed nothing, which would cost a slug card in every preview
/// until somebody regenerated it.
fn edit(
    project: &mut Project,
    root: &std::path::Path,
    id: &AssetId,
    brief: &Brief,
) -> Result<Option<String>, String> {
    let asset = project
        .assets
        .iter_mut()
        .find(|asset| &asset.id == id)
        .ok_or_else(|| format!("no asset `{id}` in this project"))?;
    if !asset.kind.is_generated() {
        return Err(format!(
            "`{id}` is a {} asset and has no brief — only the generated kinds are \
             made from one",
            named(asset)
        ));
    }
    // A ticket is money already committed, and the video it is for is named
    // after the brief that was sent. Editing now would file whatever arrives
    // under the new brief's name, so the way through is to collect first.
    if asset.state == Some(GenerationState::Queued) {
        return Err(format!(
            "`{id}` is queued at the provider and nothing was changed — collect it \
             first (generate with collect), then edit the brief; it has been paid for \
             either way"
        ));
    }
    apply(asset, root, brief)
}

/// Writes the brief onto the asset, having checked the asset takes that kind.
fn apply(
    asset: &mut Asset,
    root: &std::path::Path,
    brief: &Brief,
) -> Result<Option<String>, String> {
    match brief {
        Brief::Prompt(prompt) => {
            if !asset.kind.is_prompted() {
                return Err(mismatch(asset, "prompt", "recipe"));
            }
            if asset.prompt.as_deref() == Some(prompt.as_str()) {
                return Ok(None);
            }
            asset.prompt = Some(prompt.clone());
        }
        Brief::Recipe(recipe) => {
            if !asset.kind.is_synthesized() {
                return Err(mismatch(asset, "recipe", "prompt"));
            }
            recipe
                .check()
                .map_err(|problem| format!("`{recipe}` is not a usable recipe path: {problem}"))?;
            if !recipe.resolve(root).is_file() {
                return Err(format!(
                    "no recipe at `{recipe}` — nothing was changed. synth_new writes a \
                     starter one, and synth_write replaces an existing one"
                ));
            }
            if asset.recipe.as_ref() == Some(recipe) {
                return Ok(None);
            }
            asset.recipe = Some(recipe.clone());
        }
    }
    Ok(Some(restate(asset, brief.field())))
}

/// The brief this asset's kind does not take, refused by name.
fn mismatch(asset: &Asset, given: &str, takes: &str) -> String {
    format!(
        "`{}` is a {} asset: its brief is a {takes} and not a {given}, and nothing \
         was changed",
        asset.id,
        named(asset)
    )
}

/// What an asset's `kind` is called *in the document*.
///
/// Read out of the format rather than off `Debug`, which would answer
/// `GeneratedVideo` to somebody looking at `"kind": "generated_video"` in the
/// file they are being told about.
fn named(asset: &Asset) -> String {
    serde_json::to_value(asset.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{:?}", asset.kind))
}

/// Moves the asset's state on, and says what it now is.
///
/// `generated` becomes `stale`, which is the whole transition this tool exists
/// for. Everything else stays put: `stale` is already stale, and a `sketch` was
/// never realised, so calling it stale would claim a generation that never
/// happened. Both are already states GO acts on, so nothing is lost by leaving
/// them — and the reply says which one it is either way, because "it is stale
/// now" is worth nothing if it is sometimes untrue.
fn restate(asset: &mut Asset, field: &str) -> String {
    let was = asset.state;
    if was == Some(GenerationState::Generated) {
        asset.state = Some(GenerationState::Stale);
    }
    let id = &asset.id;
    match was {
        Some(GenerationState::Generated) => format!(
            "`{id}`: {field} changed, generated → **stale**. The file on disk is the \
             previous brief's, so the cut shows a slug card until the next generate \
             redoes it."
        ),
        Some(GenerationState::Sketch) => format!(
            "`{id}`: {field} changed, and it stays **sketch** — it has never been \
             generated, so there is nothing stale about it. The next generate \
             realises it."
        ),
        _ => format!(
            "`{id}`: {field} changed, and it stays **stale**. The next generate \
             redoes it."
        ),
    }
}

/// What a call that asked for what was already there comes back with.
fn unchanged(brief: &Brief, id: &AssetId) -> Reply {
    format!(
        "`{id}`: its {} already reads exactly that, so nothing was written and its \
         state is untouched.",
        brief.field()
    )
    .into()
}

/// A required string argument, refused by name when it is missing or blank.
fn required<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    optional(arguments, key).ok_or_else(|| format!("`{key}` is required"))
}

/// A string argument that may be absent, with a blank one counting as absent.
fn optional<'a>(arguments: &'a Value, key: &str) -> Option<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
}
