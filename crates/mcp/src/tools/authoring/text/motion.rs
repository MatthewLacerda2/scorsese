//! The two blocks that make a caption move on its own: how it reveals, and the
//! figure it counts.
//!
//! Read straight into the document's own types, so an unknown field or a word
//! that is not a unit is refused with the same reason a hand-written
//! `project.json` would get — there is one grammar for these, and it is the
//! format's.

use scorsese_core::{Counter, Reveal, TextStyle};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Adds whichever of the two blocks the call carries to `style`, starting one
/// from the defaults when nothing else about the look was said.
pub(super) fn apply(
    arguments: &Value,
    style: Option<TextStyle>,
) -> Result<Option<TextStyle>, String> {
    let reveal: Option<Reveal> = block(arguments, "reveal")?;
    let number: Option<Counter> = block(arguments, "number")?;
    if reveal.is_none() && number.is_none() {
        return Ok(style);
    }
    let mut style = style.unwrap_or_default();
    style.reveal = reveal.or(style.reveal);
    style.number = number.or(style.number);
    Ok(Some(style))
}

fn block<T: DeserializeOwned>(arguments: &Value, key: &str) -> Result<Option<T>, String> {
    let Some(value) = arguments.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|error| format!("`{key}`: {error}"))
}

/// The schema of the `reveal` block.
pub(super) fn reveal_property() -> Value {
    serde_json::json!({
        "type": "object",
        "description": "How the text arrives a piece at a time — the word-by-word caption, \
                        the typewriter, the list that builds. This says HOW; WHEN is a \
                        `reveal` keyframe track on the clip going 0 (nothing) to 1 (all), \
                        whose easing shapes each piece's own entrance (`back_out` makes \
                        every word pop). Every field has a default, so a `reveal` track \
                        alone already reveals word by word; send this only to change that.",
        "properties": {
            "unit": { "type": "string", "enum": ["char", "word", "line"],
                "description": "What the text is cut into. Default `word`. An emoji is one \
                                `char`, never halves." },
            "rise": { "type": "number",
                "description": "How far below its place each piece starts, as a fraction \
                                of the text's own size. Default 0.2; 0 fades in place, \
                                negative drops in from above." },
            "stagger": { "type": "number",
                "description": "How far one piece gets through its entrance before the \
                                next begins, 0 to 1. Default 0.5; 1 is one after another \
                                (a typewriter with unit `char` and rise 0), 0 all at once." }
        }
    })
}

/// The schema of the `number` block.
pub(super) fn number_property() -> Value {
    serde_json::json!({
        "type": "object",
        "description": "A figure written where the text says `{n}` — `{n} partitions`, \
                        `R$ {n} mi`, `{n}%`. It counts when the clip has a `number` \
                        keyframe track (0 to 144 as it lands), never showing a figure past \
                        the keyframes it is between, and the line holds still while it \
                        counts. The text must contain `{n}`.",
        "properties": {
            "value": { "type": "number",
                "description": "The figure shown when no `number` track animates it — \
                                usually the one the count ends on. Default 0." },
            "decimals": { "type": "integer",
                "description": "Digits after the decimal mark, 0 to 6. Default 0." },
            "locale": { "type": "string", "enum": ["en", "pt-BR"],
                "description": "Whose separators: `en` writes 1,234.5 and `pt-BR` writes \
                                1.234,5. Default `en`; the document's choice, never the \
                                machine's." },
            "grouping": { "type": "boolean",
                "description": "Whether thousands are grouped. Default true; false is what \
                                a year wants." }
        }
    })
}
