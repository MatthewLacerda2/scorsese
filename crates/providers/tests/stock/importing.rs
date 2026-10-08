//! Importing: an ordinary asset, the right rendition, one failure alone.

use scorsese_core::AssetKind;
use scorsese_providers::stock::{Choice, Medium, cache_dir, import, previews, search};

use crate::common::project;
use crate::fake::{Fake, Probe, query};

#[test]
fn an_import_is_an_ordinary_video_asset_named_for_its_source() {
    let (root, mut project) = project("stock-import");
    let cache = cache_dir(&root);
    let library = Fake::with(10);
    let choices = [
        Choice {
            medium: Medium::Video,
            id: 4,
        },
        Choice {
            medium: Medium::Video,
            id: 99,
        },
        Choice {
            medium: Medium::Video,
            id: 5,
        },
    ];
    let answers = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choices,
        (1920, 1080),
        &Probe,
    );
    assert_eq!(answers.len(), 3);
    let first = answers[0].as_ref().unwrap();
    assert_eq!(first.imported.id.to_string(), "pixabay-4");
    assert_eq!(first.rendition.name, "medium");
    assert!(first.fills);
    assert!(answers[1].is_err(), "99 is not there");
    assert!(answers[2].is_ok(), "a failure costs none of the others");

    let asset = project.asset(&first.imported.id).unwrap();
    assert_eq!(asset.kind, AssetKind::Video);
    assert_eq!(
        asset.path.as_ref().unwrap().as_str(),
        "assets/pixabay-4.mp4"
    );
    assert!(root.join("assets/pixabay-4.mp4").is_file());
    let downloads = std::fs::read_dir(cache.join("downloads")).unwrap().count();
    assert_eq!(downloads, 0, "the download is gone once copied in");
}

#[test]
fn a_4k_render_takes_the_large_rendition() {
    let (root, mut project) = project("stock-4k");
    let library = Fake::with(3);
    let choice = [Choice {
        medium: Medium::Video,
        id: 2,
    }];
    let answer = import(
        &mut project,
        &root,
        &cache_dir(&root),
        &library,
        &choice,
        (3840, 2160),
        &Probe,
    );
    assert_eq!(answer[0].as_ref().unwrap().rendition.name, "large");
}

#[test]
fn previews_are_downloaded_once() {
    let (root, _) = project("stock-previews");
    let cache = cache_dir(&root);
    let library = Fake::with(5);
    let found = search(&cache, &library, &query("cat"), 1).unwrap();
    let first = previews(&cache, &library, &found.candidates);
    assert!(first.iter().all(Option::is_some));
    previews(&cache, &library, &found.candidates);
    assert_eq!(library.downloads.get(), 5);
}
