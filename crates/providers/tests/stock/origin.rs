//! An import records where its file came from (#1237): the library, the id,
//! the author, the page and the licence, from the result already in hand.

use scorsese_core::{Source, StockLibrary};
use scorsese_providers::stock::{Choice, Imports, LICENCE_NAME, Medium, cache_dir, import};

use crate::common::project;
use crate::fake::{Fake, probe};

#[test]
fn an_import_records_its_library_author_page_and_licence() {
    let (root, mut project) = project("stock-origin");
    let (cache, library) = (cache_dir(&root), Fake::with(10));
    let choice = [Choice {
        medium: Medium::Video,
        id: 4,
    }];
    let frame = (1920, 1080);
    let answers = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choice,
        frame,
        &probe(),
    );
    let first = answers[0].as_ref().expect("video 4 is there");
    assert!(first.recorded);
    let asset = project.asset(&first.imported.id).expect("it is an asset");
    let expected = Source {
        library: StockLibrary::Pixabay,
        id: "4".to_owned(),
        author: Some("someone".to_owned()),
        page_url: Some("https://pixabay.com/videos/id-4/".to_owned()),
        licence: Some(LICENCE_NAME.to_owned()),
    };
    assert_eq!(asset.source.as_ref(), Some(&expected));
    project.validate().expect("and the project is still valid");
}

#[test]
fn a_held_file_without_a_source_learns_it_and_is_saved() {
    let (root, mut project) = project("stock-origin-held");
    let (cache, library) = (cache_dir(&root), Fake::with(10));
    let choice = [Choice {
        medium: Medium::Video,
        id: 4,
    }];
    let frame = (1920, 1080);
    let answers = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choice,
        frame,
        &probe(),
    );
    let id = answers[0]
        .as_ref()
        .expect("video 4 is there")
        .imported
        .id
        .clone();
    // As though the same bytes had been brought in by hand, before #1237.
    project
        .assets
        .iter_mut()
        .find(|a| a.id == id)
        .expect("held")
        .source = None;

    let again = import(
        &mut project,
        &root,
        &cache,
        &library,
        &choice,
        frame,
        &probe(),
    );
    let again = Imports::new(&choice, again, frame);
    assert!(
        again.needs_save(),
        "the held file learned where it came from"
    );
    let asset = project.asset(&id).expect("still one asset");
    assert_eq!(asset.source.as_ref().map(|s| s.id.as_str()), Some("4"));
}
