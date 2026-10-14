//! v48 → v49: `expressive` is Eleven v4, and `natural` retires (#1248).

use serde_json::Value;

/// v48 → v49: every `natural` line becomes an `expressive` one.
///
/// At v48 `natural` was `eleven_v4` and `expressive` was `eleven_v3`. At v49
/// `expressive` is `eleven_v4`, so two names would stand for one model — a
/// choice with no difference — and `natural` goes. A `natural` line asks the
/// vendor for exactly what it did, under the other name.
///
/// What moves is the line's brief fingerprint, which hashes the document's
/// word: a `natural` line already spoken is spoken again, once, at the next
/// run. `natural` was offered for less than a day before this version, so
/// that is the cost accepted rather than a lookup kept for ever.
///
/// The `expressive` lines a v48 document already holds are untouched here.
/// They now ask for v4 instead of v3, which is the decision itself, and their
/// fingerprints do not move, so the v3 files already paid for keep playing.
pub(super) fn natural_retires(document: &mut Value) -> Result<(), String> {
    let Some(assets) = document.get_mut("assets").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for asset in assets {
        if asset.get("kind").and_then(Value::as_str) != Some("generated_audio") {
            continue;
        }
        let Some(model) = asset.pointer_mut("/speech/model") else {
            continue;
        };
        if model.as_str() == Some("natural") {
            *model = Value::from("expressive");
        }
    }
    Ok(())
}
