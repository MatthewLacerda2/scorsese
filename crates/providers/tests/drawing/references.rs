//! Reference pictures: read off disk, hashed by their bytes, and — when they
//! are generated stills themselves — drawn first.

use scorsese_core::{Asset, AssetId, AssetKind, ImageRequest, ProjectPath};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, generate};

use crate::common::write;
use crate::mock::Mock;
use crate::{asset_mut, sketched};

/// Names `references` on the still `id`.
fn naming(project: &mut scorsese_core::Project, id: &AssetId, references: &[&str]) {
    asset_mut(project, id).image = Some(ImageRequest {
        reference_images: references.iter().map(|r| AssetId::new(*r)).collect(),
        ..ImageRequest::default()
    });
}

#[test]
fn an_imported_reference_is_sent_and_its_bytes_are_the_brief() {
    let (dir, mut project, id) = sketched("referenced", "the same face, smiling");
    write(&dir, "assets/face.png", "first face");
    project.assets.push(Asset::imported(
        AssetId::new("face"),
        AssetKind::Image,
        ProjectPath::new("assets/face.png"),
    ));
    naming(&mut project, &id, &["face"]);
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let sent = provider.drawn.borrow()[0].reference_images.clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].bytes, b"first face");
    assert_eq!(sent[0].mime_type, "image/png");

    // Another picture under the same name is a different brief.
    write(&dir, "assets/face.png", "second face");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert_eq!(provider.requests(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_sheet_still_a_sketch_waits_for_the_next_run() {
    let (dir, mut project, id) = sketched("chained", "the character, running");
    project.assets.push(Asset::sketch(
        AssetId::new("sheet"),
        AssetKind::GeneratedImage,
        "a character sheet",
    ));
    naming(&mut project, &id, &["sheet"]);
    let provider = Mock::willing();

    let first = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(
        matches!(first[0].1, Outcome::Incomplete { .. }),
        "{:?}",
        first[0].1
    );
    assert!(matches!(first[1].1, Outcome::Generated { .. }));

    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert!(
        matches!(second[0].1, Outcome::Generated { .. }),
        "{:?}",
        second[0].1
    );
    assert_eq!(
        provider.requests(),
        2,
        "the sheet once, then the still drawn from it"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Objects go first, then characters; and the kind is never sent, so moving a
/// picture from one kind to another is the same request and draws nothing.
#[test]
fn every_kind_is_sent_in_order_and_the_kind_is_not_the_brief() {
    let (dir, mut project, id) = sketched("kinds", "the hero holding the cup");
    for (name, bytes) in [("cup", "a cup"), ("hero", "a face")] {
        write(&dir, &format!("assets/{name}.png"), bytes);
        project.assets.push(Asset::imported(
            AssetId::new(name),
            AssetKind::Image,
            ProjectPath::new(format!("assets/{name}.png")),
        ));
    }
    asset_mut(&mut project, &id).image = Some(ImageRequest {
        reference_images: vec![AssetId::new("cup")],
        character_images: vec![AssetId::new("hero")],
        ..ImageRequest::default()
    });
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let sent: Vec<Vec<u8>> = provider.drawn.borrow()[0]
        .reference_images
        .iter()
        .map(|still| still.bytes.clone())
        .collect();
    assert_eq!(sent, vec![b"a cup".to_vec(), b"a face".to_vec()]);

    naming(&mut project, &id, &["cup", "hero"]);
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    assert_eq!(
        provider.requests(),
        1,
        "the same pictures, in the same order"
    );
    std::fs::remove_dir_all(&dir).ok();
}
