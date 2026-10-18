//! What a preview played in pieces asks for (#1448): the sound one file at
//! full quality whatever is asked, a picture piece keeping its range, and a
//! preview played whole keyed as it was before pieces.

use scorsese_server::renders::{PreviewAsk, Settings};
use serde_json::{Value, json};

#[test]
fn the_sound_is_one_file_whatever_the_quality_and_a_piece_keeps_its_range() {
    let preview = |ask: Value| {
        Settings::from_preview(&serde_json::from_value::<PreviewAsk>(ask).expect("an ask"))
    };
    let half = preview(json!({ "stream": "sound", "quality": "half" })).unwrap();
    assert_eq!(
        half,
        preview(json!({ "stream": "sound", "quality": "quarter" })).unwrap()
    );
    assert_eq!(
        (half.container.as_str(), half.resolution.as_deref()),
        ("m4a", None)
    );
    let piece = preview(json!({ "stream": "picture", "range": "30:60" })).unwrap();
    assert_eq!(piece.range.as_deref(), Some("30:60"));
    assert!(
        preview(json!({ "stream": "both" }))
            .unwrap_err()
            .contains("picture or sound")
    );
    // A preview played whole keeps the key it had before pieces existed.
    let whole = serde_json::to_value(preview(json!({})).unwrap()).unwrap();
    assert!(whole.get("stream").is_none(), "{whole}");
}
