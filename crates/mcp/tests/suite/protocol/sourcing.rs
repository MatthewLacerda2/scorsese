//! `project_assets` says where a stock import came from (#1237).

use super::fixture::{project, read};
use crate::protocol::{call, said};
use serde_json::json;

#[test]
fn a_stock_asset_lists_its_origin_and_a_brought_one_none() {
    let dir = project("sourcing");
    let mut document = read(&dir);
    let assets = document["assets"].as_array_mut().expect("an assets table");
    assets.push(json!({
        "id": "pixabay-39009", "kind": "video", "path": "assets/pixabay-39009.mp4",
        "source": { "library": "pixabay", "id": "39009", "author": "someone",
                    "page_url": "https://pixabay.com/videos/id-39009/",
                    "licence": "Pixabay Content License" }
    }));
    let written = serde_json::to_string_pretty(&document).expect("JSON");
    std::fs::write(dir.join("project.json"), written).expect("write project.json");

    let (text, failed) = said(&call("project_assets", json!({ "project": dir })));
    assert!(!failed, "{text}");
    let row = text
        .lines()
        .find(|line| line.starts_with("pixabay-39009"))
        .unwrap_or_else(|| panic!("no row for the stock asset in {text}"));
    assert!(
        row.ends_with(
            "\tfrom Pixabay 39009, by someone, under the Pixabay Content License — \
             https://pixabay.com/videos/id-39009/"
        ),
        "{row}"
    );
    assert_eq!(text.matches("\tfrom ").count(), 1, "{text}");
    std::fs::remove_dir_all(dir).ok();
}
