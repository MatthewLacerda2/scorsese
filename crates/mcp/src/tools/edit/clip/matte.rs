//! A clip's track matte over the wire: `matte`.
//!
//! The same bargain [`super::light`]'s objects make: an object sets what it
//! names — `{ "clip": "c-wipe" }` masks the clip by `c-wipe`, `{ "invert": true
//! }` on a clip that already has a matte turns it inside out — and `false`
//! removes it, because `null` already means "not given".

use scorsese_core::{Clip, ClipId, Matte};
use serde_json::{Value, json};

/// The argument's schema, for `clip_set`'s own.
pub(super) fn schema() -> (&'static str, Value) {
    (
        "matte",
        json!({
            "description": "Show this clip only through another clip's picture — a track \
                            matte, for a wipe, an iris or footage through the letters of a \
                            title. The matte clip is used only as a mask and is no longer \
                            drawn itself; whatever animates it (a scale growing, a blur \
                            softening its edge) animates the reveal. It must be on the same \
                            timeline (both inside one group, or both outside), must not be \
                            this clip, and must not have a matte of its own. An object sets \
                            the fields it names and keeps the rest; `false` removes the \
                            matte, and the matte clip is drawn again. Picture only.",
            "type": ["object", "boolean"],
            "properties": {
                "clip": {
                    "type": "string",
                    "description": "Id of the clip whose picture is the mask. Required \
                                    when the clip has no matte yet."
                },
                "invert": {
                    "type": "boolean",
                    "description": "Show this clip where the matte is not, rather than where \
                                    it is: a hole cut the matte's shape. Default false."
                }
            },
            "additionalProperties": false
        }),
    )
}

/// Applies the `matte` argument to `clip`, if it was given; what it did, in
/// words.
pub(super) fn apply(clip: &mut Clip, arguments: &Value) -> Result<Option<String>, String> {
    let Some(given) = arguments.get("matte").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if given == &Value::Bool(false) {
        clip.matte = None;
        return Ok(Some("no matte".to_owned()));
    }
    let Some(fields) = given.as_object() else {
        return Err("`matte` is an object or `false`".to_owned());
    };
    if let Some(key) = fields
        .keys()
        .find(|key| !["clip", "invert"].contains(&key.as_str()))
    {
        return Err(format!("`matte` takes `clip` and `invert`, not `{key}`"));
    }
    let named = match fields.get("clip") {
        None => None,
        Some(id) => Some(ClipId::new(id.as_str().ok_or("`matte.clip` is a clip id")?)),
    };
    let invert = match fields.get("invert") {
        None => None,
        Some(flag) => Some(flag.as_bool().ok_or("`matte.invert` is true or false")?),
    };
    let matte = match (clip.matte.take(), named) {
        (_, Some(id)) => Matte::new(id),
        (Some(had), None) => had,
        (None, None) => {
            return Err("`matte` needs a `clip` to mask this one by".to_owned());
        }
    };
    let matte = Matte {
        invert: invert.unwrap_or(matte.invert),
        ..matte
    };
    let said = format!(
        "shown {} clip `{}`",
        if matte.invert { "outside" } else { "through" },
        matte.clip
    );
    clip.matte = Some(matte);
    Ok(Some(said))
}
