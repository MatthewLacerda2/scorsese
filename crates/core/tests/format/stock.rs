//! An imported file's stock `source`, with every field set, for
//! [`super::document`]'s asset.

use scorsese_core::{Source, StockLibrary};

/// Where the document's one asset came from.
pub(crate) fn source() -> Source {
    Source {
        library: StockLibrary::Pixabay,
        id: "39009".to_owned(),
        author: Some("someone".to_owned()),
        page_url: Some("https://pixabay.com/videos/id-39009/".to_owned()),
        licence: Some("Pixabay Content License".to_owned()),
    }
}
