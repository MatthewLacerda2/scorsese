//! The voice tools' arguments, read the same way by `voice_design` and
//! `voice_clone`.

use serde_json::Value;

/// A string argument, blank counting as absent.
pub(super) fn text(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// The seed, refused rather than rounded when it is not a whole number.
pub(super) fn seed(arguments: &Value) -> Result<Option<u32>, String> {
    match arguments.get("seed") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("seed: {value} is not a whole number a seed can be")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_seed_is_a_whole_number_or_nothing() {
        assert_eq!(seed(&json!({})), Ok(None));
        assert_eq!(seed(&json!({ "seed": 7 })), Ok(Some(7)));
        assert!(seed(&json!({ "seed": 1.5 })).is_err());
        assert!(seed(&json!({ "seed": -1 })).is_err());
    }

    #[test]
    fn blank_text_is_absent() {
        assert_eq!(text(&json!({ "keep": "  " }), "keep"), None);
        assert_eq!(
            text(&json!({ "keep": " a " }), "keep"),
            Some("a".to_owned())
        );
    }
}
