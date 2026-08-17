//! What the compiled-in blob holds: the whole vendored tree, and the words
//! that make it findable.

use std::path::PathBuf;

use scorsese_compositor::icon;

/// The vendored directory, from this crate's own manifest.
fn vendored() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons")
}

/// Every `.svg` in the vendored tree is an entry in the blob, and no entry is
/// anything else. This is the check that the committed blob was built from the
/// committed tree: they are two files that have to agree, and a bump that
/// forgot to re-run the converter is exactly what it catches.
#[test]
fn the_catalogue_is_the_vendored_tree() {
    let mut vendored: Vec<String> = std::fs::read_dir(vendored().join("lucide"))
        .expect("the vendored icons")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|kind| kind == "svg"))
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_string))
        .collect();
    vendored.sort();

    let compiled: Vec<&str> = icon::all().iter().map(|icon| icon.name()).collect();
    assert_eq!(compiled.len(), vendored.len(), "one entry per icon file");
    assert!(
        compiled.iter().zip(&vendored).all(|(a, b)| a == b),
        "the blob holds the same names as the tree, in the same order"
    );
}

/// The pinned release is recorded in one place and reported from another —
/// `VERSION` is what the converter reads, the blob is what the compositor
/// answers with, and a blob rebuilt from a different tree would say so.
#[test]
fn the_version_is_the_pinned_one() {
    let recorded = std::fs::read_to_string(vendored().join("VERSION")).expect("the version");
    assert_eq!(icon::version(), recorded.trim());
    assert!(!icon::version().is_empty(), "a blob that read");
}

/// The licence travels with the icons. It is ISC, it needs no notice in an
/// MP4, and the file being here is the whole of what we owe upstream.
#[test]
fn the_licence_is_vendored_beside_them() {
    let licence = std::fs::read_to_string(vendored().join("LICENSE")).expect("the licence");
    assert!(licence.contains("ISC License"), "the ISC text");
    assert!(licence.contains("Lucide Icons"), "whose it is");
}

/// A name is looked up, and one that is not in the set is answered rather than
/// approximated. Nothing downstream can correct a symbol quietly swapped for a
/// near-match.
#[test]
fn a_name_finds_its_icon_and_nothing_else_does() {
    assert!(icon::find("clapperboard").is_some(), "a vendored name");
    assert!(icon::find("play").is_some(), "another");
    assert!(icon::find("clapperboards").is_none(), "not quite a name");
    assert!(icon::find("").is_none(), "no name at all");
    assert!(icon::find("Clapperboard").is_none(), "names are lowercase");
}

/// The metadata is carried, not discarded. Seventeen hundred names is not a
/// list anybody reads, so the tags are what a search over the set is made of.
#[test]
fn an_icon_carries_the_words_it_is_found_by() {
    let clapperboard = icon::find("clapperboard").expect("a vendored icon");
    let tags = clapperboard.tags();
    assert!(tags.contains(&"cinema"), "found by what it means: {tags:?}");
    assert!(tags.contains(&"film"), "and by the other words for it");
    assert_eq!(clapperboard.categories(), ["multimedia"]);
}

/// Most icons have tags and some have none; what would be wrong is the whole
/// set arriving bare, which is what discarding the metadata would look like.
#[test]
fn the_set_is_tagged_and_categorised_throughout() {
    let tagged = icon::all().iter().filter(|i| !i.tags().is_empty()).count();
    let filed = icon::all()
        .iter()
        .filter(|icon| !icon.categories().is_empty())
        .count();
    let total = icon::all().len();
    assert!(tagged * 2 > total, "{tagged} of {total} icons carry tags");
    assert!(filed * 2 > total, "{filed} of {total} are in a category");
}
