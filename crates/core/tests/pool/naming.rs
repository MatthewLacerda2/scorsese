//! What a listing calls an asset no clip shows, and the words a row is in
//! (#1115, #1166): `gc` keeps what another asset or a page names, so the
//! listing must not call it unused, and a row speaks the document's words
//! rather than Rust's.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    Asset, AssetHealth, AssetId, AssetKind, GenerationState, HashCheck, NamedBy, asset_status,
    import_asset,
};

fn named_by(rows: &[scorsese_core::AssetStatus], id: &AssetId) -> Option<NamedBy> {
    let row = rows.iter().find(|row| &row.id == id).expect("listed");
    assert_eq!(row.clip_count, 0);
    row.named_by.clone()
}

#[test]
fn a_still_a_shot_starts_from_lists_as_named_by_the_shot_not_unused() {
    let (dir, mut project) = new_project("naming-shot");
    let source = source_file(&dir, "photo.png", b"a picture");
    let photo =
        import_asset(&mut project, &dir, &source, None, &StubProbe::image()).expect("import");
    let mut shot = Asset::sketch(AssetId::new("intro"), AssetKind::GeneratedVideo, "a cat");
    shot.video.get_or_insert_default().first_image = Some(photo.clone());
    project.assets.push(shot);

    let rows = asset_status(&project, &dir, HashCheck::Skip);
    let named = named_by(&rows, &photo).expect("the shot names it");
    assert_eq!(named.to_string(), "named by shot `intro`");
    assert_eq!(
        named_by(&rows, &AssetId::new("intro")),
        None,
        "nothing names the shot"
    );
}

#[test]
fn a_picture_a_page_names_lists_as_named_by_the_page() {
    let (dir, mut project) = new_project("naming-page");
    let source = source_file(&dir, "photo.png", b"a picture");
    let photo =
        import_asset(&mut project, &dir, &source, None, &StubProbe::image()).expect("import");
    let card = source_file(&dir, "card.html", br#"<img src="../assets/photo.png">"#);
    let page = import_asset(
        &mut project,
        &dir,
        &card,
        None,
        &StubProbe::failing("a page"),
    )
    .expect("import the page");

    let rows = asset_status(&project, &dir, HashCheck::Skip);
    assert_eq!(named_by(&rows, &photo), Some(NamedBy::Page(page.clone())));
    assert_eq!(
        NamedBy::Page(page).to_string(),
        "named by page `card`",
        "the page's id, as the document has it"
    );
}

#[test]
fn every_kind_is_said_as_the_document_spells_it() {
    for kind in AssetKind::ALL {
        let spelled = serde_json::to_value(kind).expect("serialise");
        assert_eq!(spelled.as_str(), Some(kind.to_string().as_str()));
    }
    let sketch = serde_json::to_value(GenerationState::Sketch).expect("serialise");
    assert_eq!(
        sketch.as_str(),
        Some(GenerationState::Sketch.to_string().as_str())
    );
}

#[test]
fn a_health_is_said_in_words_not_rust() {
    let awaiting = AssetHealth::Awaiting(GenerationState::Sketch).to_string();
    assert_eq!(awaiting, "awaiting generation (sketch)");
    let changed = AssetHealth::HashMismatch {
        recorded: "a".repeat(64),
        found: "b".repeat(64),
    };
    assert_eq!(changed.to_string(), "changed since import");
}
