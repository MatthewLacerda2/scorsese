//! v45 → v46: stills at parity with Google's image models (#893).
//!
//! The one step since the rule began that rewrites anything, because the
//! version changed what an absence means, not only what may be written.

use serde_json::{Map, Value};

/// How many objects a Flash still may name at v46. At v45 it took fourteen
/// references of no stated kind; Google's page caps that at ten objects and
/// four characters.
const FLASH_OBJECTS: usize = 10;

/// v45 → v46: Nano Banana 2.1 and Pro, the long strips, thinking levels, and
/// references split by kind.
///
/// Two rewrites, each so that a v45 still asks for exactly what it did:
///
/// - **The default model moved** from `flash` to `nano_banana_2.1`, which is
///   better and cheaper per picture. A v45 still that named no model was a
///   Flash still, so it is written out as one — `image.model: "flash"`, the
///   block created if it was absent. Its brief's fingerprint does not move:
///   the model always went in resolved.
/// - **References split into objects, characters and styles.** A v45 Flash
///   still could name fourteen of no stated kind, and Flash takes ten objects
///   and four characters. Any past the tenth become `character_images`, which
///   hands the vendor the same pictures in the same order — the kind is never
///   sent, so the fingerprint does not move either. Lite took fourteen at v45
///   and takes fourteen objects at v46, so a Lite still is untouched.
pub(super) fn stills_parity_arrives(document: &mut Value) -> Result<(), String> {
    let Some(assets) = document.get_mut("assets").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for asset in assets {
        if asset.get("kind").and_then(Value::as_str) != Some("generated_image") {
            continue;
        }
        let Some(asset) = asset.as_object_mut() else {
            continue;
        };
        let image = asset
            .entry("image")
            .or_insert_with(|| Value::Object(Map::new()));
        if image.is_null() {
            *image = Value::Object(Map::new());
        }
        let Some(image) = image.as_object_mut() else {
            return Err(String::from("a generated_image's `image` is not an object"));
        };
        let model = image.entry("model").or_insert_with(|| Value::from("flash"));
        if model.as_str() != Some("flash") {
            continue;
        }
        let Some(references) = image
            .get_mut("reference_images")
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        if references.len() > FLASH_OBJECTS {
            let characters = references.split_off(FLASH_OBJECTS);
            image.insert(String::from("character_images"), Value::Array(characters));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
