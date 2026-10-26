//! A stock import's origin is part of the listing (#1237).

use super::list;
use crate::common::documents::{self, assets};

#[test]
fn a_stock_import_says_where_it_came_from_under_its_row() {
    let plain = assets::video("pixabay-39009");
    let sourced = plain.replacen(
        r#""kind": "video","#,
        r#""kind": "video", "source": { "library": "pixabay", "id": "39009",
           "author": "someone", "licence": "Pixabay Content License",
           "page_url": "https://pixabay.com/videos/id-39009/" },"#,
        1,
    );
    assert_ne!(plain, sourced, "the fixture's shape moved");
    let dir = documents::project(
        "stock-source",
        &[sourced, assets::video("brought")],
        &[documents::clip("c1", "pixabay-39009", 0)],
    );
    documents::media_file(&dir, "pixabay-39009", b"");
    documents::media_file(&dir, "brought", b"");
    let run = list(&dir, &[]).ok();

    run.says(
        "  from Pixabay 39009, by someone, under the Pixabay Content License — \
         https://pixabay.com/videos/id-39009/",
    );
    assert_eq!(run.output.matches("  from ").count(), 1, "{}", run.output);
    std::fs::remove_dir_all(&dir).ok();
}
