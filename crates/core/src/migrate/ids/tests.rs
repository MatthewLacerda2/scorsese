use serde_json::{Value, json};

use crate::migrate::parse;
use crate::{AssetId, Project, SCHEMA_VERSION};

/// A v55 document whose ids break the v56 rule, named from every place a v55
/// document names an asset: a timeline clip, a group's member, a shot's three
/// still fields, a generated still's two lists and a sequence's stills.
fn v55() -> Value {
    json!({
        "schema_version": 55,
        "name": "Ids that are file names",
        "timeline_fps": { "num": 30, "den": 1 },
        "aspect": "16:9",
        "assets": [
            { "id": "shot-one", "kind": "image", "path": "assets/a.png" },
            { "id": "shot one", "kind": "image", "path": "assets/b.png" },
            { "id": "shot/one", "kind": "image", "path": "assets/c.png" },
            { "id": "Praça Nova", "kind": "image", "path": "assets/d.png" },
            { "id": "../../escaped", "kind": "image", "path": "assets/e.png" },
            { "id": "%%%", "kind": "image", "path": "assets/f.png" },
            { "id": "take: 2", "kind": "generated_video", "state": "sketch", "prompt": "rain",
              "video": { "model": "standard", "resolution": "720p", "seconds": 8,
                         "first_image": "shot one", "last_image": "shot/one",
                         "reference_images": ["Praça Nova"] } },
            { "id": "poster", "kind": "generated_image", "state": "sketch", "prompt": "a poster",
              "image": { "reference_images": ["shot/one"],
                         "character_images": ["../../escaped"] } },
            { "id": "flip book", "kind": "image_sequence",
              "sequence": { "stills": ["shot one", "%%%", "shot-one"] } },
            { "id": "card #1", "kind": "group", "group": { "tracks": [
                { "id": "g1", "kind": "video", "clips": [
                    { "id": "inner", "asset": "Praça Nova", "start": 0, "duration": 30 }
                ]}
            ]}}
        ],
        "tracks": [
            { "id": "v1", "kind": "video", "clips": [
                { "id": "c1", "asset": "shot one", "start": 0, "duration": 30 },
                { "id": "c2", "asset": "card #1", "start": 30, "duration": 30 },
                { "id": "c3", "asset": "flip book", "start": 60, "duration": 30 },
                { "id": "c4", "asset": "take: 2", "start": 90, "duration": 30 }
            ]}
        ]
    })
}

fn ids(project: &Project) -> Vec<&str> {
    project
        .assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect()
}

/// Every id that breaks the rule gets a hyphen for each run of characters it
/// may not hold, trimmed, `asset` when nothing is left, and the first free
/// `-2`, `-3`… when that is somebody else's; every id that keeps it is left.
#[test]
fn a_v55_document_s_bad_ids_are_renamed_and_it_validates() {
    let (project, from) = parse(&v55().to_string()).expect("a v55 document migrates");
    assert_eq!(from, Some(55));
    assert_eq!(project.schema_version, SCHEMA_VERSION);
    project.validate().expect("and it validates");
    assert_eq!(
        ids(&project),
        [
            "shot-one",
            "shot-one-2",
            "shot-one-3",
            "Pra-a-Nova",
            "escaped",
            "asset",
            "take-2",
            "poster",
            "flip-book",
            "card-1",
        ]
    );
}

/// Every reference follows its asset, wherever the document names it.
#[test]
fn every_reference_names_the_renamed_asset() {
    let (project, _) = parse(&v55().to_string()).expect("a v55 document migrates");
    let id = AssetId::new;
    let clips: Vec<&str> = project.tracks[0]
        .clips
        .iter()
        .map(|clip| clip.asset.as_str())
        .collect();
    assert_eq!(clips, ["shot-one-2", "card-1", "flip-book", "take-2"]);
    let shot = project
        .asset(&id("take-2"))
        .expect("the shot")
        .video_request();
    assert_eq!(shot.first_image, Some(id("shot-one-2")));
    assert_eq!(shot.last_image, Some(id("shot-one-3")));
    assert_eq!(shot.reference_images, [id("Pra-a-Nova")]);
    let still = project
        .asset(&id("poster"))
        .expect("the still")
        .image_request();
    assert_eq!(still.reference_images, [id("shot-one-3")]);
    assert_eq!(still.character_images, [id("escaped")]);
    let book = project.asset(&id("flip-book")).expect("the sequence");
    let stills = &book.sequence.as_ref().expect("its block").stills;
    assert_eq!(stills, &[id("shot-one-2"), id("asset"), id("shot-one")]);
    let card = project.asset(&id("card-1")).expect("the group");
    let member = &card.group.as_ref().expect("its tracks").tracks[0].clips[0];
    assert_eq!(member.asset, id("Pra-a-Nova"));
}

/// A renamed id that would run past the cap is cut short enough to take its
/// suffix, and a document whose ids all keep the rule is left as it was.
#[test]
fn a_long_id_is_cut_to_the_cap_and_a_good_document_is_untouched() {
    let long = format!("{} x", "a".repeat(200));
    let mut document = v55();
    document["assets"][5]["id"] = json!(long);
    document["assets"][8]["sequence"]["stills"][1] = json!(long);
    let (project, _) = parse(&document.to_string()).expect("migrates");
    project.validate().expect("and it validates");
    let cut = &project.assets[5].id;
    assert_eq!(cut.as_str(), "a".repeat(crate::MAX_ASSET_ID_LENGTH));

    let good = json!({
        "schema_version": 55, "name": "Good", "timeline_fps": { "num": 30, "den": 1 },
        "assets": [{ "id": "a.b_C-1", "kind": "image", "path": "assets/a.png" }],
        "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c1", "asset": "a.b_C-1", "start": 0, "duration": 30 }]}]
    });
    let mut migrated = good.clone();
    super::rename_bad_ids(&mut migrated).expect("a step");
    assert_eq!(migrated, good);
}
