//! Animations: searched like footage, imported beside the pages.

use scorsese_providers::stock::{
    Choice, Medium, StockError, cache_dir, footage, import, keep, named_in, search,
};

use crate::common::project;
use crate::fake::{Fake, LOTTIE, Probe, query};

#[test]
fn an_animation_lands_beside_the_pages_and_not_in_the_assets() {
    let (root, mut project) = project("stock-lottie");
    let cache = cache_dir(&root);
    let library = Fake::with(20);
    let found = search(
        &cache,
        &library,
        &scorsese_providers::stock::Query {
            medium: Medium::Lottie,
            ..query("cat waving")
        },
        1,
    )
    .unwrap();
    let listing: Vec<String> = found.candidates.iter().map(|one| one.says()).collect();
    assert!(
        listing[0].starts_with("lottie 1  \"Wave 1\"  2s at 30 fps"),
        "{listing:?}"
    );
    let named = named_in(&listing.join("\n"));
    assert_eq!(
        named[0].medium,
        Medium::Lottie,
        "a listing names its animations back"
    );

    let kept = keep(&root, &cache, &library, &[3, 99, 13, 4]);
    let first = kept[0].as_ref().unwrap();
    assert_eq!(first.path, "pages/lottie-3.json");
    assert_eq!(first.file, "lottie-3.json");
    assert_eq!(
        (first.size, first.fps, first.frames),
        ((512.0, 512.0), 30.0, 60.0)
    );
    assert!((first.seconds() - 2.0).abs() < 1e-9);
    assert!(!first.reused);
    assert_eq!(
        std::fs::read_to_string(root.join("pages/lottie-3.json")).unwrap(),
        LOTTIE
    );
    assert!(
        matches!(kept[1], Err(StockError::NotFound { .. })),
        "99 is not there"
    );
    assert!(
        matches!(kept[2], Err(StockError::NotLottie { .. })),
        "13 is not a Lottie"
    );
    assert!(kept[3].is_ok(), "a failure costs none of the others");
    assert!(
        !root.join("pages/lottie-13.json").exists(),
        "nothing that is not one is kept"
    );
    assert!(
        project
            .assets
            .iter()
            .all(|asset| !asset.id.as_str().contains("lottie"))
    );

    let again = keep(&root, &cache, &library, &[3]);
    assert!(
        again[0].as_ref().unwrap().reused,
        "the same file is not written twice"
    );

    let refused = import(
        &mut project,
        &root,
        &cache,
        &library,
        &[Choice {
            medium: Medium::Lottie,
            id: 3,
        }],
        (1920, 1080),
        &Probe,
    );
    assert!(matches!(refused[0], Err(StockError::NotMedia { id: 3 })));
}

#[test]
fn an_animation_is_looked_through_from_its_own_video() {
    let (root, _) = project("stock-lottie-look");
    let cache = cache_dir(&root);
    let library = Fake::with(5);
    let (candidate, file) = footage(&cache, &library, Medium::Lottie, 2).unwrap();
    assert_eq!(candidate.medium, Medium::Lottie);
    assert!(file.ends_with("looks/lottie-2.mp4"));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "bytes of https://cdn.example.invalid/2.mp4"
    );
    assert!(
        footage(&cache, &library, Medium::Image, 2).is_err(),
        "a picture does not move"
    );
}
