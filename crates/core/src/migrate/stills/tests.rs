//! The v45 → v46 step held to the rule every step is held to: a v45 document,
//! migrated, loads and validates, and asks for what it asked for before.

use serde_json::json;

use crate::migrate::parse;

/// v45 → v46: a v45 still that named no model stays a Flash still, and one
/// with more than ten references keeps every picture, the rest as characters.
#[test]
fn a_v45_document_s_stills_stay_flash_and_keep_every_reference() {
    let many: Vec<String> = (0..12).map(|n| format!("ref-{n}")).collect();
    let mut assets: Vec<_> = many
        .iter()
        .map(|id| json!({ "id": id, "kind": "generated_image", "state": "sketch", "prompt": id }))
        .collect();
    assets.push(
        json!({ "id": "bare", "kind": "generated_image", "state": "sketch",
                        "prompt": "a lighthouse" }),
    );
    assets.push(
        json!({ "id": "sheet", "kind": "generated_image", "state": "sketch",
                        "prompt": "a crowd", "image": { "reference_images": [] } }),
    );
    assets.push(
        json!({ "id": "cheap", "kind": "generated_image", "state": "sketch",
                        "prompt": "a cat", "image": { "model": "lite" } }),
    );
    let document = json!({
        "schema_version": 45,
        "name": "Before Nano Banana 2.1",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": assets,
        "tracks": []
    });
    let mut document = document;
    document["assets"][13]["image"]["reference_images"] = json!(many);
    let (project, from) = parse(&document.to_string()).expect("a v45 document migrates");
    assert_eq!(from, Some(45));
    project.validate().expect("and it validates");
    let request = |id: &str| {
        project
            .asset(&crate::AssetId::new(id))
            .expect(id)
            .image_request()
    };
    assert_eq!(request("bare").model, crate::ImageModel::Flash);
    assert_eq!(request("cheap").model, crate::ImageModel::Lite);
    let sheet = request("sheet");
    assert_eq!(sheet.model, crate::ImageModel::Flash);
    assert_eq!(sheet.reference_images.len(), 10);
    assert_eq!(sheet.character_images.len(), 2);
    let order: Vec<String> = sheet.references().map(ToString::to_string).collect();
    assert_eq!(order, many, "the same pictures, in the same order");
}
