//! v49 → v50: Nano Banana 2.1 is the only image model (#1308).

use serde_json::{Map, Value};

/// The most objects a still takes at v50 (Lite took fourteen at v49).
const OBJECTS: usize = 10;

/// The most characters a still takes at v50 (Pro took five at v49).
const CHARACTERS: usize = 4;

/// v49 → v50: every still asks Nano Banana 2.1, so `image.model` goes.
///
/// Each `generated_image`'s `image` block is rewritten to what 2.1 takes:
///
/// - **`model` is dropped**, whatever it named: one model is not a choice.
/// - **`resolution: "0.5K"` becomes `"1K"`**, the smallest size 2.1 draws
///   (0.5K was Flash's alone).
/// - **`style_images` is dropped** (Pro's alone; 2.1 takes no styles).
/// - **Objects past the tenth move into `character_images`** while it has
///   room for four, and the rest are dropped; **characters past the fourth
///   are dropped**. Lite took fourteen objects and Pro five characters.
///
/// **A still already drawn stays drawn.** Its `state` and its `media` are
/// left exactly as they were, so the next GO neither redraws nor bills it. A
/// still drawn with another model gets a new brief fingerprint — its model
/// line now reads 2.1's — but a fingerprint only decides what a sketch or a
/// stale still lands as, and the file it already plays is the one it keeps.
pub(in crate::migrate) fn one_image_model(document: &mut Value) -> Result<(), String> {
    let Some(assets) = document.get_mut("assets").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for asset in assets {
        if asset.get("kind").and_then(Value::as_str) != Some("generated_image") {
            continue;
        }
        let Some(image) = asset.get_mut("image").filter(|image| !image.is_null()) else {
            continue;
        };
        let Some(image) = image.as_object_mut() else {
            return Err(String::from("a generated_image's `image` is not an object"));
        };
        image.remove("model");
        image.remove("style_images");
        if image.get("resolution").and_then(Value::as_str) == Some("0.5K") {
            image.insert(String::from("resolution"), Value::from("1K"));
        }
        fit_references(image);
    }
    Ok(())
}

/// Objects past [`OBJECTS`] into characters while there is room, then both
/// kinds cut to their limits — the pictures kept stay in the order they were.
fn fit_references(image: &mut Map<String, Value>) {
    let mut objects = list(image, "reference_images");
    let mut characters = list(image, "character_images");
    if objects.len() > OBJECTS {
        let past = objects.split_off(OBJECTS);
        let room = CHARACTERS.saturating_sub(characters.len());
        characters.extend(past.into_iter().take(room));
    }
    characters.truncate(CHARACTERS);
    for (field, ids) in [
        ("reference_images", objects),
        ("character_images", characters),
    ] {
        if ids.is_empty() {
            image.remove(field);
        } else {
            image.insert(String::from(field), Value::Array(ids));
        }
    }
}

/// The ids under `field`, empty when it is absent or not a list.
fn list(image: &Map<String, Value>, field: &str) -> Vec<Value> {
    image
        .get(field)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
