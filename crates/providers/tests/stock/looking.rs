//! What both surfaces show before an import: the labelled previews a search
//! sheet tiles, and the footage a look takes frames of (#1288).

use scorsese_providers::stock::{
    Medium, StockError, cache_dir, footage, labelled_previews, search,
};

use crate::common::project;
use crate::fake::{Fake, query};

/// A photo is refused before the library is asked: there is nothing in it to
/// look through.
#[test]
fn a_still_is_not_looked_through_and_nothing_is_asked() {
    let (root, _) = project("stock-look-still");
    let library = Fake::with(10);
    let refused = footage(&cache_dir(&root), &library, Medium::Image, 3);
    assert!(
        matches!(refused, Err(StockError::Still { id: 3 })),
        "{refused:?}"
    );
    assert_eq!((library.ones.get(), library.downloads.get()), (0, 0));
}

/// Each preview carries its number in the reply and its id, in order.
#[test]
fn previews_are_labelled_in_the_order_of_the_reply() {
    let (root, _) = project("stock-labelled");
    let cache = cache_dir(&root);
    let library = Fake::with(10);
    let found = search(&cache, &library, &query("cat"), 1).unwrap();
    let labels: Vec<String> = labelled_previews(&cache, &library, &found.candidates)
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    assert_eq!(labels[..2], ["1. 1", "2. 2"]);
    assert_eq!(labels.len(), found.candidates.len());
}
