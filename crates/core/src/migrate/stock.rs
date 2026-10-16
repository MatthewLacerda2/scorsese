//! v52 → v53: an imported file may say which stock library it came from
//! (#1237).

use serde_json::Value;

/// v52 → v53: the optional `source` block on `video` and `image` assets.
///
/// Nothing to rewrite: the field is new and absent in every v52 document,
/// and absent means a file the user brought. A `pixabay-<id>` file name is
/// deliberately **not** read back into a `source` — the author and the page
/// were never stored, and half a record would read as the whole of one.
pub(super) fn stock_sources_arrive(_: &mut Value) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::migrate::parse;

    /// A v52 stock import arrives with no `source`, its file name unread.
    #[test]
    fn a_v52_stock_import_arrives_without_a_source() {
        let document = json!({
            "schema_version": 52,
            "name": "Before stock sources",
            "timeline_fps": { "num": 30, "den": 1 },
            "aspect": "16:9",
            "assets": [
                { "id": "pixabay-39009", "kind": "video",
                  "path": "assets/pixabay-39009.mp4" }
            ],
            "tracks": []
        });
        let (project, from) = parse(&document.to_string()).expect("a v52 document migrates");
        assert_eq!(from, Some(52));
        project.validate().expect("and it validates");
        assert_eq!(project.assets[0].source, None);
    }
}
