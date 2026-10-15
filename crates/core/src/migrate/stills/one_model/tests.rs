//! The v49 → v50 step held to the rule every step is held to: a v49 document,
//! migrated, loads and validates, and asks for the nearest thing Nano Banana
//! 2.1 draws — and a still already drawn stays drawn.

use serde_json::json;

use crate::migrate::parse;

/// v49 → v50: every still loses its model and what only another model took,
/// and keeps as many of its pictures as 2.1 takes, in their order.
#[test]
fn a_v49_document_s_stills_all_ask_nano_banana_2_1() {
    let ids = |prefix: &str, n: usize| -> Vec<String> {
        (0..n).map(|i| format!("{prefix}{i}")).collect()
    };
    let mut assets: Vec<_> = ids("o", 14)
        .into_iter()
        .chain(ids("c", 5))
        .map(|id| json!({ "id": id, "kind": "image", "path": format!("assets/{id}.png") }))
        .collect();
    assets.extend([
        json!({ "id": "small", "kind": "generated_image", "state": "sketch", "prompt": "a",
                "image": { "model": "flash", "resolution": "0.5K", "thinking": "minimal" } }),
        json!({ "id": "lite", "kind": "generated_image", "state": "sketch", "prompt": "b",
                "image": { "model": "lite", "reference_images": ids("o", 14) } }),
        json!({ "id": "pro", "kind": "generated_image", "state": "sketch", "prompt": "c",
                "image": { "model": "pro", "resolution": "4K", "aspect": "21:9",
                           "reference_images": ids("o", 6), "character_images": ids("c", 5),
                           "style_images": ["o0"] } }),
        json!({ "id": "full", "kind": "generated_image", "state": "sketch", "prompt": "d",
                "image": { "model": "lite", "reference_images": ids("o", 12),
                           "character_images": ids("c", 3) } }),
        json!({ "id": "bare", "kind": "generated_image", "state": "sketch", "prompt": "e" }),
    ]);
    let document = json!({
        "schema_version": 49,
        "name": "Before one image model",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": assets,
        "tracks": []
    });
    let (project, from) = parse(&document.to_string()).expect("a v49 document migrates");
    assert_eq!(from, Some(49));
    project.validate().expect("and it validates");
    let request = |id: &str| {
        project
            .asset(&crate::AssetId::new(id))
            .expect(id)
            .image_request()
    };
    let named =
        |list: &[crate::AssetId]| -> Vec<String> { list.iter().map(ToString::to_string).collect() };

    let small = request("small");
    assert_eq!(small.size(), crate::ImageResolution::K1);
    assert_eq!(small.thinking, Some(crate::ImageThinking::Minimal));

    let lite = request("lite");
    assert_eq!(named(&lite.reference_images), ids("o", 10));
    assert_eq!(named(&lite.character_images), ids("o", 14)[10..]);

    let pro = request("pro");
    assert_eq!(pro.size(), crate::ImageResolution::K4);
    assert_eq!(pro.aspect, crate::ImageAspect::Cinema);
    assert_eq!(named(&pro.reference_images), ids("o", 6));
    assert_eq!(named(&pro.character_images), ids("c", 4));

    let full = request("full");
    assert_eq!(named(&full.reference_images), ids("o", 10));
    let mut characters = ids("c", 3);
    characters.push(String::from("o10"));
    assert_eq!(named(&full.character_images), characters);

    assert_eq!(request("bare"), crate::ImageRequest::default());
}

/// v49 → v50 keeps a still already drawn drawn: a Flash still that was
/// `generated` is still `generated`, with the same file, and nothing in the
/// project is left to draw — so GO never redraws or re-bills it.
#[test]
fn a_v49_still_already_drawn_with_flash_stays_drawn() {
    let document = json!({
        "schema_version": 49,
        "name": "Drawn with Flash",
        "timeline_fps": { "num": 30, "den": 1 },
        "assets": [
            { "id": "poster", "kind": "generated_image", "state": "generated",
              "prompt": "a poster", "path": "generated/poster-abc.jpg",
              "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "image": { "model": "flash", "resolution": "0.5K" } }
        ],
        "tracks": []
    });
    let (project, _) = parse(&document.to_string()).expect("a v49 document migrates");
    project.validate().expect("and it validates");
    let poster = &project.assets[0];
    assert_eq!(poster.state, Some(crate::GenerationState::Generated));
    assert_eq!(
        poster.path.as_ref().map(crate::ProjectPath::as_str),
        Some("generated/poster-abc.jpg")
    );
    assert!(
        project.assets.iter().all(|asset| !asset.needs_generation()),
        "nothing is planned for drawing"
    );
}
