//! The rest of a prompted brief — a shot's `video`, a still's `image`, a
//! line's `speech` — as `asset_set` takes it (#826).
//!
//! Each block is read by the document's own deserialiser, so a misspelt field
//! is refused in the words a hand-written `project.json` would get, and it is
//! *merged*: on an asset that has a block, the fields named change and the
//! rest stay, the way every other argument of this tool behaves. Before this,
//! the only way to write one was the whole document — and the record #779 read
//! has a model spending three refused `project_write`s guessing the speech
//! block's shape. So the schema spells the fields out, values and all.

use schemars::{Schema, SchemaGenerator};
use scorsese_core::{Asset, AssetKind, Inline};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::Arguments;

/// A free sketch of a prompted `kind`, from the arguments.
pub(super) fn sketch(kind: AssetKind, arguments: &Arguments) -> Result<Inline, String> {
    let prompt = arguments
        .prompt
        .clone()
        .filter(|text| !text.trim().is_empty())
        .ok_or("`prompt` is required: the sentence a provider will be paid to read")?;
    let (mut video, mut image, mut speech) = (None, None, None);
    merged(&mut video, arguments.video.as_ref(), "video")?;
    merged(&mut image, arguments.image.as_ref(), "image")?;
    merged(&mut speech, arguments.speech.as_ref(), "speech")?;
    Ok(Inline::Sketch {
        kind,
        prompt,
        video,
        image,
        speech,
    })
}

/// Merges the block the asset's kind takes into it, naming the block when the
/// call carried one.
pub(super) fn merge(
    asset: &mut Asset,
    arguments: &Arguments,
) -> Result<Option<&'static str>, String> {
    match asset.kind {
        AssetKind::GeneratedVideo => merged(&mut asset.video, arguments.video.as_ref(), "video"),
        AssetKind::GeneratedImage => merged(&mut asset.image, arguments.image.as_ref(), "image"),
        AssetKind::GeneratedAudio => merged(&mut asset.speech, arguments.speech.as_ref(), "speech"),
        _ => Ok(None),
    }
}

/// The fields `given` names, laid over `block` — `null` returning one to its
/// default — and read back as the document's type.
///
/// A block that ends up every default stays absent when it was absent: the
/// format says the two are the same brief, so writing it out would mark a
/// generated asset stale for a request nobody changed.
fn merged<T>(
    block: &mut Option<T>,
    given: Option<&Value>,
    key: &'static str,
) -> Result<Option<&'static str>, String>
where
    T: Serialize + DeserializeOwned + Default + PartialEq + Clone,
{
    let Some(given) = given.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Value::Object(fields) = given else {
        return Err(format!(
            "`{key}` is an object of the brief's fields, not {given}"
        ));
    };
    let current = serde_json::to_value(block.clone().unwrap_or_default())
        .map_err(|error| format!("`{key}`: {error}"))?;
    let Value::Object(mut current) = current else {
        return Err(format!("`{key}`: the block is not an object"));
    };
    for (field, value) in fields {
        if value.is_null() {
            current.remove(field);
        } else {
            current.insert(field.clone(), value.clone());
        }
    }
    let read: T = serde_json::from_value(Value::Object(current))
        .map_err(|error| format!("`{key}`: {error}"))?;
    if block.is_some() || read != T::default() {
        *block = Some(read);
    }
    Ok(Some(key))
}

/// A brief block's schema: what it is for, and its fields.
fn block(description: &str, properties: Value) -> Schema {
    let schema = serde_json::json!({
        "type": "object",
        "description": format!(
            "{description} Every field has a default; on an existing asset the fields \
             named change, the rest stay, and `null` returns one to its default. \
             `guide project-format` has the combinations project_check refuses."
        ),
        "properties": properties,
        "additionalProperties": false
    });
    Schema::try_from(schema).expect("a brief block's schema is an object")
}

/// The `video` argument's schema.
pub(super) fn video_schema(_: &mut SchemaGenerator) -> Schema {
    block(
        "(generated_video) The rest of the shot's brief. 1080p, 4k, reference images, \
         or a first and last image together are eight seconds only.",
        serde_json::json!({
            "model": { "enum": ["standard", "fast", "lite"], "description": "Default fast; standard is the full model and costs the most; lite costs less and takes no reference_images and no 4k." },
            "resolution": { "enum": ["720p", "1080p", "4k"], "description": "Default 1080p." },
            "seconds": { "enum": [4, 6, 8] },
            "aspect": { "enum": ["16:9", "9:16"] },
            "first_image": { "type": "string", "description": "A still's asset id: the frame the shot opens on." },
            "last_image": { "type": "string", "description": "A still's asset id, with first_image: where the shot ends." },
            "reference_images": { "type": "array", "items": { "type": "string" }, "maxItems": 3,
                "description": "Asset ids of stills of a subject that should look like itself." }
        }),
    )
}

/// The `image` argument's schema.
pub(super) fn image_schema(_: &mut SchemaGenerator) -> Schema {
    block(
        "(generated_image) The rest of the still's brief. `resolution` is the money \
         lever: 2K covers a 1080p frame with room to push in.",
        serde_json::json!({
            "model": { "enum": ["flash", "lite"], "description": "Default flash; lite costs half and draws 1K only." },
            "resolution": { "enum": ["0.5K", "1K", "2K", "4K"] },
            "aspect": { "enum": ["16:9", "9:16", "1:1", "4:3", "3:4", "3:2", "2:3", "5:4", "4:5", "21:9"] },
            "reference_images": { "type": "array", "items": { "type": "string" }, "maxItems": 14,
                "description": "Asset ids of image or generated_image assets to keep a subject looking like itself." }
        }),
    )
}

/// The `speech` argument's schema.
pub(super) fn speech_schema(_: &mut SchemaGenerator) -> Schema {
    block(
        "(generated_audio) How the line is said.",
        serde_json::json!({
            "model": { "enum": ["expressive", "standard", "fast"], "description": "Default fast, half the price of the other two." },
            "voice_id": { "type": "string", "description": "The vendor's id for the voice — voices lists them. No default is possible, so a line needs one before it is generated." },
            "language": { "type": "string", "description": "ISO 639-1 (`en`, `pt`), pinning what the model would infer." },
            "seed": { "type": "integer", "minimum": 0, "description": "For a reading that comes back the same way twice; best-effort." }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{ImageRequest, SpeechRequest, VideoRequest};

    /// Every value a schema offers is one the document's type reads — the
    /// schema is hand-written, so this is what keeps it from drifting.
    fn every_value_reads<T: DeserializeOwned>(schema: &Schema) {
        let value = schema.as_value();
        for (field, property) in value["properties"].as_object().expect("properties") {
            for offered in property["enum"].as_array().into_iter().flatten() {
                let one = serde_json::json!({ field: offered });
                assert!(
                    serde_json::from_value::<T>(one).is_ok(),
                    "`{field}`: {offered} is offered and not read"
                );
            }
        }
    }

    #[test]
    fn the_offered_values_are_the_formats() {
        let generator = &mut SchemaGenerator::default();
        every_value_reads::<VideoRequest>(&video_schema(generator));
        every_value_reads::<ImageRequest>(&image_schema(generator));
        every_value_reads::<SpeechRequest>(&speech_schema(generator));
    }

    /// The merge keeps what it was not told about, and `null` restores a
    /// default.
    #[test]
    fn a_block_merges_field_by_field() {
        let mut speech = Some(SpeechRequest {
            voice_id: Some("v1".to_owned()),
            seed: Some(4),
            ..SpeechRequest::default()
        });
        let given = serde_json::json!({ "language": "pt", "seed": null });
        merged(&mut speech, Some(&given), "speech").expect("merges");
        let speech = speech.expect("still there");
        assert_eq!(speech.voice_id.as_deref(), Some("v1"));
        assert_eq!(speech.language.as_deref(), Some("pt"));
        assert_eq!(speech.seed, None);
    }

    /// A misspelt field is refused in the format's words, naming the block.
    #[test]
    fn an_unknown_field_is_refused_by_name() {
        let mut speech: Option<SpeechRequest> = None;
        let given = serde_json::json!({ "voice": "v1" });
        let refused = merged(&mut speech, Some(&given), "speech").expect_err("unknown");
        assert!(refused.starts_with("`speech`:"), "{refused}");
        assert!(speech.is_none());
    }
}
