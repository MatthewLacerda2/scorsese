//! v55 → v56: an asset id is a safe file name.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::asset::{AssetId, tidy, unique};
use crate::path::ProjectPath;
use crate::project::GENERATED_DIR;

/// v55 → v56: every asset id that breaks [`AssetId::check`]'s rule is renamed,
/// and every reference to it follows.
///
/// Up to v55 an id could hold anything, and it becomes a file name — a paid
/// output's, a partial bake's, a page's — so a `/` put it in a subfolder and a
/// `..` outside the project. The new id is the old one [`tidy`]-ed (each run of
/// characters it may not hold becomes one `-`, leading dots and hyphens go,
/// `asset` when nothing is left, cut in its middle to the cap), suffixed `-2`,
/// `-3`… when another asset has it already in any case. An id that keeps the
/// rule is left exactly as written, and is never the one suffixed.
///
/// A v55 document names an asset from a clip's `asset` (the timeline's and
/// each group's), a shot's `first_image`, `last_image` and
/// `reference_images`, a generated still's `reference_images` and
/// `character_images`, and a sequence's `stills`; each follows the rename, and
/// so does the asset's own `path` when it is a paid output named for it
/// ([`carried`]). The file itself is [`super::carry`]'s.
pub(in crate::migrate) fn rename_bad_ids(document: &mut Value) -> Result<(), String> {
    let renames = renames(document);
    if renames.is_empty() {
        return Ok(());
    }
    let follow = |value: &mut Value| {
        if let Some(new) = value.as_str().and_then(|old| renames.get(old)) {
            *value = Value::from(new.as_str());
        }
    };
    for asset in each(document, "assets") {
        let renamed = asset.get("id").and_then(Value::as_str);
        if let Some((old, new)) = renamed.and_then(|old| Some((old, renames.get(old)?))) {
            let path = asset.get("path").and_then(Value::as_str);
            let moved = path.and_then(|path| carried(path, old, new));
            asset["id"] = Value::from(new.as_str());
            if let Some(moved) = moved {
                asset["path"] = Value::from(moved);
            }
        }
        for (block, fields) in REFERENCES {
            let Some(block) = asset.get_mut(*block) else {
                continue;
            };
            for field in *fields {
                match block.get_mut(*field) {
                    Some(Value::Array(ids)) => ids.iter_mut().for_each(follow),
                    Some(id) => follow(id),
                    None => {}
                }
            }
        }
        if let Some(group) = asset.get_mut("group") {
            each(group, "tracks").for_each(|track| clips(track, &follow));
        }
    }
    each(document, "tracks").for_each(|track| clips(track, &follow));
    Ok(())
}

/// Where a paid output named for `old` is named for `new`: a `path` of
/// `generated/<old>-<brief hash>.<ext>` follows the rename, since a run looks
/// for an asset's take under its id and [`super::carry`] links the file there.
/// Any other path — an imported file's, a page's — is left as written.
///
/// Only a path the folder can resolve moves — the one [`super::carry`] can
/// link — so a path is never moved to a name with no file behind it.
fn carried(path: &str, old: &str, new: &str) -> Option<String> {
    ProjectPath::new(path).check().ok()?;
    let rest = path.strip_prefix(&format!("{GENERATED_DIR}/{old}-"))?;
    super::carry::is_brief_name(rest).then(|| format!("{GENERATED_DIR}/{new}-{rest}"))
}

/// Each asset-level block of a v55 document that names assets, and the
/// fields in it that do.
const REFERENCES: &[(&str, &[&str])] = &[
    ("video", &["first_image", "last_image", "reference_images"]),
    ("image", &["reference_images", "character_images"]),
    ("sequence", &["stills"]),
];

/// Old id to new, for every id that breaks the rule, in document order.
fn renames(document: &Value) -> HashMap<String, String> {
    let ids: Vec<&str> = document
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|asset| asset.get("id")?.as_str())
        .collect();
    let bad = |id: &str| AssetId::new(id).check().is_err();
    // Compared without case: on a case-blind disk `Pra-a` and `pra-a` name one
    // file, so a rename never lands on another id's other spelling.
    let mut taken: HashSet<String> = ids
        .iter()
        .filter(|id| !bad(id))
        .map(|id| id.to_ascii_lowercase())
        .collect();
    let mut renames = HashMap::new();
    for old in ids.iter().copied().filter(|id| bad(id)) {
        if renames.contains_key(old) {
            continue;
        }
        let new = unique(&tidy(old), ids.len(), |candidate| {
            taken.contains(&candidate.to_ascii_lowercase())
        });
        taken.insert(new.to_ascii_lowercase());
        renames.insert(old.to_owned(), new);
    }
    renames
}

/// The array under `key`, element by element; nothing when there is none.
fn each<'a>(value: &'a mut Value, key: &str) -> impl Iterator<Item = &'a mut Value> {
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
}

/// Every clip on `track` has its `asset` followed.
fn clips(track: &mut Value, follow: &impl Fn(&mut Value)) {
    for clip in each(track, "clips") {
        if let Some(asset) = clip.get_mut("asset") {
            follow(asset);
        }
    }
}

#[cfg(test)]
mod tests;
