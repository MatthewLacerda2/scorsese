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

mod bake;
mod export;
mod import;
mod kit;
mod recipes;
mod set;
#[cfg(test)]
mod stopping;
mod survey;

pub(super) use bake::Bake;
pub(super) use export::Export;
pub(super) use import::Import;
pub(super) use kit::Kit;
pub(super) use set::Set;
pub(super) use survey::Survey;

use schemars::JsonSchema;
use scorsese_core::ProjectPath;
use scorsese_providers::synth::{self, Starter, kit as library};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::args::{self, Name, ProjectDir};
use super::inspect::load;
use super::{Costs, Reply, Tool};
use recipes::{RECIPE, RecipeArguments, read, recipe_path};

/// A list of strings, where `null` is the empty list.
///
/// Read as one value rather than entry by entry, so a list with a number in it
/// is refused as the argument the caller wrote and not as its third entry —
/// and a refusal says *a list of strings* rather than serde's *a sequence*.
fn strings<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    use serde::de::Error as _;
    let wrong = || D::Error::custom("a list of strings");
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(Vec::new()),
        Value::Array(entries) => entries
            .into_iter()
            .map(|entry| match entry {
                Value::String(text) => Ok(text),
                _ => Err(wrong()),
            })
            .collect(),
        _ => Err(wrong()),
    }
}

/// Start a recipe.
pub(super) struct New;

/// What a starter recipe is, when no instrument is named.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Patch,
    Song,
}

/// What `synth_new` takes.
#[derive(Deserialize, JsonSchema)]
struct NewArguments {
    project: ProjectDir,
    /// What to call it. Becomes the asset id and the recipe's file name,
    /// suffixed if that is taken.
    name: Name,
    /// `patch` for one instrument playing one note — an effect. `song` for an
    /// arrangement — a score. Default `patch`.
    kind: Option<Kind>,
    /// Start from a library instrument instead — `kick`, `epiano`; synth_kit
    /// lists them. The recipe is one note of it, its patch copied in for you to
    /// edit. Takes the place of `kind`.
    instrument: Option<String>,
}

impl args::Arguments for NewArguments {}

impl Tool for New {
    fn name(&self) -> &'static str {
        "synth_new"
    }

    fn description(&self) -> &'static str {
        "Start a new sound: writes a starter recipe into recipes/ and adds the \
         synth_audio asset that points at it. The starter makes a sound as \
         written, so bake it and listen before changing anything. Given an \
         `instrument` from synth_kit, the recipe is one note of that instrument, \
         copied in. Costs nothing — synthesis needs no key, no network and no \
         money."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<NewArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: NewArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let starter =
            match (arguments.instrument.as_deref(), arguments.kind) {
                (Some(name), _) => Starter::Kit(library::lookup(name).ok_or_else(|| {
                    format!("there is no `{name}` in the kit — synth_kit lists it")
                })?),
                (None, Some(Kind::Song)) => Starter::Song,
                (None, None | Some(Kind::Patch)) => Starter::Patch,
            };

        let id = synth::create(&mut project, dir, arguments.name.as_str(), starter)
            .map_err(|error| format!("{error}"))?;
        project
            .save(dir)
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

/// Parse a recipe on disk without rendering it.
///
/// It reads the file, not a document, so it is not a dry run of `synth_write`
/// — that one parses before it writes. It is for a recipe that reached disk
/// some other way: a local client's own file tools, which over stdio are often
/// how an agent edits one (#784 kept it for that).
pub(super) struct Check;

impl Tool for Check {
    fn name(&self) -> &'static str {
        "synth_check"
    }

    fn description(&self) -> &'static str {
        "Check a recipe file that was changed outside synth_write — edited with \
         your own file tools, or copied in — and say what it is, without \
         rendering it. Not needed before synth_write or after synth_new or \
         synth_set: those parse what they write and refuse what is not a \
         recipe. Milliseconds rather than the seconds a bake takes."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<RecipeArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: RecipeArguments = args::parse(arguments)?;
        let (file, relative) = recipe_path(arguments.project.dir(), &arguments.recipe)?;
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

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<RecipeArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: RecipeArguments = args::parse(arguments)?;
        let (file, _) = recipe_path(arguments.project.dir(), &arguments.recipe)?;
        read(&file).map(Into::into)
    }
}

/// Replace the recipe.
pub(super) struct Write;

/// What `synth_write` takes.
#[derive(Deserialize, JsonSchema)]
struct WriteArguments {
    project: ProjectDir,
    #[schemars(description = RECIPE)]
    recipe: Name,
    /// The complete recipe JSON to write. Not a patch — whatever is here
    /// replaces the file.
    document: String,
}

impl args::Arguments for WriteArguments {}

impl Tool for Write {
    fn name(&self) -> &'static str {
        "synth_write"
    }

    fn description(&self) -> &'static str {
        "Replace a recipe file with the document given. Parsed before it is \
         written — a document that is not a recipe is refused and the file on \
         disk is left as it was. Writing a recipe makes its asset stale by \
         arithmetic: the bake is named for the recipe's hash, so the next \
         synth_bake redoes it and nothing has to be marked. The synthesiser's \
         own version is in that name too, so a bake never outlives the code \
         that made it. A track whose patch is a library name — \"kit:kick\", \
         see synth_kit — gets a copy of that instrument written in its place, \
         so the recipe on disk carries the patch itself."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<WriteArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: WriteArguments = args::parse(arguments)?;
        let (file, relative) = recipe_path(arguments.project.dir(), &arguments.recipe)?;
        // Written as given; only a document with nothing in it is refused, the
        // way a missing one is.
        let document = arguments.document.as_str();
        if document.trim().is_empty() {
            return Err("`document` is required".to_owned());
        }
        // Parsed before it is written, for the same reason `project_write`
        // validates: a recipe that is not a recipe makes every later bake fail
        // with a message about a file nobody remembers editing.
        //
        // A `kit:` name is copied in first, because the copy is what gets
        // written: the document on disk never depends on the library.
        let expanded = library::expand(document).map_err(|problem| {
            format!("refused, nothing written — {relative} would not parse: {problem}")
        })?;
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("creating {}: {error}", parent.display()))?;
        }
        scorsese_core::write::atomically(&file, &expanded.json)
            .map_err(|error| format!("writing {relative}: {error}"))?;
        let copied = if expanded.copied.is_empty() {
            String::new()
        } else {
            format!(
                " Copied in from the kit: {}. They are this recipe's own now; \
                 synth_read shows them.",
                expanded.copied.join(", ")
            )
        };
        Ok(format!(
            "{relative} written — a {} recipe. Its asset is stale now; \
             synth_bake redoes it.{copied}",
            expanded.recipe.kind()
        )
        .into())
    }
}
