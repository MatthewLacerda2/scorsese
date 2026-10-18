//! What a piece of a preview played in pieces is (#1448): rendered with
//! sound a browser can be handed losslessly, kept as the MP4 it is remuxed
//! into, and keyed by its range — while a preview played whole is keyed as
//! it was before pieces.

use scorsese_server::renders::{PreviewAsk, Settings};
use serde_json::{Value, json};

#[test]
fn a_piece_is_rendered_with_pcm_kept_as_mp4_and_keyed_by_its_range() {
    let preview = |ask: Value| {
        Settings::from_preview(&serde_json::from_value::<PreviewAsk>(ask).expect("an ask"))
    };
    let piece = preview(json!({ "range": "30:60" })).unwrap();
    assert!(piece.piece());
    assert_eq!(
        (
            piece.container.as_str(),
            piece.video_codec.as_deref(),
            piece.audio_codec.as_str()
        ),
        ("avi", Some("h264"), "pcm_s16le")
    );
    assert_eq!(
        (piece.extension(), piece.content_type()),
        ("mp4", "video/mp4")
    );
    assert_eq!(piece.range.as_deref(), Some("30:60"));
    assert_ne!(piece, preview(json!({ "range": "60:90" })).unwrap());
    assert!(
        preview(json!({ "range": "60:30" }))
            .unwrap_err()
            .contains("range")
    );

    // A preview played whole is no piece, and keys as it did before pieces.
    let whole = preview(json!({})).unwrap();
    assert!(!whole.piece());
    assert_eq!(
        (whole.container.as_str(), whole.extension()),
        ("mp4", "mp4")
    );
    let stored = serde_json::to_value(&whole).unwrap();
    assert!(stored.get("range").is_none(), "{stored}");
}
