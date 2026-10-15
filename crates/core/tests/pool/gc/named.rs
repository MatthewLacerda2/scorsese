//! What another asset or a page names is in use, though no clip shows it
//! (#1100): collecting it would leave a project that does not load, or a page
//! drawing a broken picture.

use crate::common::{new_project, source_file};
use scorsese_core::pool::{GcError, NamedByPage, named_by_pages, remove_assets, unused_assets};
use scorsese_core::testing::StubProbe;
use scorsese_core::{Asset, AssetId, AssetKind, Project, import_asset};
use std::path::PathBuf;

/// A project holding one imported picture, `photo`, that no clip shows.
fn project_with_a_photo(label: &str) -> (PathBuf, Project, AssetId) {
    let (dir, mut project) = new_project(label);
    let source = source_file(&dir, "photo.png", b"a picture");
    let photo = import_asset(&mut project, &dir, &source, None, &StubProbe::image())
        .expect("import the photo");
    (dir, project, photo)
}

fn sketch(id: &str, kind: AssetKind) -> Asset {
    Asset::sketch(AssetId::new(id), kind, "a cat on a windowsill")
}

#[test]
fn a_still_a_shot_starts_from_is_kept_until_the_shot_is_gone() {
    let (dir, mut project, photo) = project_with_a_photo("gc-shot");
    let mut shot = sketch("shot", AssetKind::GeneratedVideo);
    shot.video.get_or_insert_default().first_image = Some(photo.clone());
    project.assets.push(shot);

    assert_eq!(unused_assets(&project), vec![AssetId::new("shot")]);
    let error = remove_assets(&mut project, &dir, std::slice::from_ref(&photo)).expect_err("used");
    assert!(
        matches!(error, GcError::StillReferenced { .. }),
        "{error:?}"
    );
    assert!(
        dir.join("assets/photo.png").is_file(),
        "nothing was deleted"
    );

    remove_assets(&mut project, &dir, &[AssetId::new("shot"), photo]).expect("shot, then still");
    assert!(project.assets.is_empty());
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn a_picture_a_generated_still_is_drawn_after_is_kept() {
    let (dir, mut project, photo) = project_with_a_photo("gc-still");
    let mut drawn = sketch("drawn", AssetKind::GeneratedImage);
    drawn.image.get_or_insert_default().reference_images = vec![photo.clone()];
    project.assets.push(drawn);

    assert_eq!(unused_assets(&project), vec![AssetId::new("drawn")]);
    let error = remove_assets(&mut project, &dir, &[photo]).expect_err("used");
    assert!(
        matches!(error, GcError::StillReferenced { .. }),
        "{error:?}"
    );
    assert!(dir.join("assets/photo.png").is_file());
}

#[test]
fn a_picture_a_page_names_survives_and_says_which_page() {
    let (dir, mut project, photo) = project_with_a_photo("gc-page");
    let source = source_file(&dir, "card.html", br#"<img src="../assets/photo.png">"#);
    let page = import_asset(
        &mut project,
        &dir,
        &source,
        None,
        &StubProbe::failing("a page"),
    )
    .expect("import the page");

    assert_eq!(
        named_by_pages(&project, &dir),
        vec![NamedByPage {
            asset: photo.clone(),
            page: page.clone(),
        }]
    );
    let error = remove_assets(&mut project, &dir, &[photo]).expect_err("named");
    assert!(matches!(error, GcError::NamedByPage { .. }), "{error:?}");
    assert!(
        dir.join("assets/photo.png").is_file(),
        "the picture survives"
    );
    assert_eq!(project.assets.len(), 2, "nothing was removed");
}

#[test]
fn a_picture_no_page_names_is_not_kept_for_one() {
    let (dir, mut project, _photo) = project_with_a_photo("gc-page-other");
    let source = source_file(&dir, "card.html", b"<img src=\"../assets/other.png\">");
    import_asset(
        &mut project,
        &dir,
        &source,
        None,
        &StubProbe::failing("a page"),
    )
    .expect("import the page");

    assert_eq!(named_by_pages(&project, &dir), vec![]);
}
