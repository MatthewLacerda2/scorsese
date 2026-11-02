//! What a shared tool may ask of the shared machine on the web (#1634): the
//! render ceilings, for the tools that draw.

use serde_json::Value;

/// Whether a call of `name` asks for no bigger a picture than the shared
/// machine draws (#1634): a `still` named by `WIDTHxHEIGHT` takes the render
/// ceilings' picture. A shape (`9:16`) or no size is a preview's pixel budget,
/// always within it.
pub(in super::super) fn within(name: &str, arguments: &Value) -> Result<(), String> {
    if name != "still" {
        return Ok(());
    }
    let sized = arguments
        .get("resolution")
        .and_then(Value::as_str)
        .and_then(|text| text.parse::<scorsese_render::Resolution>().ok());
    sized.map_or(Ok(()), |size| {
        crate::ceilings::picture(size.width(), size.height())
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::within;

    #[test]
    fn a_still_named_past_four_k_is_refused_and_a_shape_is_not() {
        let still = |resolution: &str| within("still", &json!({ "resolution": resolution }));
        assert_eq!(still("3840x2160"), Ok(()));
        assert_eq!(still("9:16"), Ok(()));
        assert_eq!(within("still", &json!({})), Ok(()));
        assert!(still("7680x4320").unwrap_err().contains("7680x4320"));
        let elsewhere = within("inspect", &json!({ "resolution": "7680x4320" }));
        assert_eq!(elsewhere, Ok(()));
    }
}
