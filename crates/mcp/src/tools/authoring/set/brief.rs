//! Changing the brief a generated asset is to be made from, and its state
//! with it — what `rebrief` was until #780 folded it in here.
//!
//! One field and one state, written together, because doing only the first is
//! the mistake this exists to make impossible: an edited brief on an asset
//! still marked `generated` reads as up to date, so the cut keeps showing the
//! previous take and nothing anywhere says otherwise.
//!
//! **The two kinds of brief stay two.** A `prompt` is a sentence handed to a
//! provider; a `recipe` is a document the project carries. Which kind takes
//! which is [`super::fields`]'s list, the same split `AssetKind::is_prompted`
//! and `is_synthesized` make and `Project::validate` holds the document to —
//! voiced before the document is written rather than after. So is the rest of
//! a prompted brief, the `video`, `image` or `speech` block: every field of it
//! is hashed with the sentence, so changing one is a new brief like any other.

use scorsese_core::{Asset, AssetId, GenerationState, Project, ProjectPath};

use super::{Arguments, requests};

/// Changes the brief of `id`, a generated asset, moves its state on, writes
/// the document, and says which state it is in now.
pub(super) fn change(
    project: &mut Project,
    dir: &std::path::Path,
    id: &AssetId,
    arguments: &Arguments,
) -> Result<String, String> {
    let asset = project
        .assets
        .iter_mut()
        .find(|asset| &asset.id == id)
        .ok_or_else(|| format!("no asset `{id}` in this project"))?;
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
    let before = asset.clone();
    let asked = apply(asset, dir, arguments)?;
    if asked.is_empty() {
        return Err(
            "nothing to change: pass `prompt` (or the brief's `video`, `image` \
                    or `speech` block) for a prompted asset, or `recipe` for a \
                    synth_audio one"
                .to_owned(),
        );
    }
    // An edit that changed nothing writes nothing: a `generated` asset marked
    // stale for it would cost a slug card in every preview until somebody
    // regenerated it.
    if *asset == before {
        return Ok(format!(
            "`{id}`: its {} already reads exactly that, so nothing was written and \
             its state is untouched.",
            asked.join(" and ")
        ));
    }
    let said = restate(asset, &asked.join(" and "));
    // Validated before the save and not after: the refusal has to arrive while
    // the file on disk is still the old one, which is what makes "nothing was
    // changed" a fact rather than a hope.
    project
        .validate()
        .map_err(|problems| format!("refused, nothing written:\n{problems}"))?;
    project
        .save(dir)
        .map_err(|error| format!("refused, nothing written: {error}"))?;
    Ok(said)
}

/// Writes whatever brief the arguments carry onto the asset, and names the
/// fields asked for. The kind was checked against them already.
fn apply(
    asset: &mut Asset,
    root: &std::path::Path,
    arguments: &Arguments,
) -> Result<Vec<&'static str>, String> {
    let mut asked = Vec::new();
    // A blank one counts as absent, and what is kept is kept as given — a
    // prompt is the provider's sentence to the letter.
    if let Some(prompt) = arguments
        .prompt
        .as_ref()
        .filter(|text| !text.trim().is_empty())
    {
        asset.prompt = Some(prompt.clone());
        asked.push("prompt");
    }
    if let Some(recipe) = arguments
        .recipe
        .as_ref()
        .filter(|text| !text.trim().is_empty())
    {
        let recipe = ProjectPath::new(recipe.clone());
        recipe
            .check()
            .map_err(|problem| format!("`{recipe}` is not a usable recipe path: {problem}"))?;
        if !recipe.resolve(root).is_file() {
            return Err(format!(
                "no recipe at `{recipe}` — nothing was changed. synth_new writes a \
                 starter one, and synth_write replaces an existing one"
            ));
        }
        asset.recipe = Some(recipe);
        asked.push("recipe");
    }
    if let Some(block) = requests::merge(asset, arguments)? {
        asked.push(block);
    }
    Ok(asked)
}

/// Moves the asset's state on, and says what it now is.
///
/// `generated` becomes `stale`, which is the whole transition this exists
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
