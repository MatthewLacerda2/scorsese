//! Arguments a tool does not take, refused by name — at any depth.
//!
//! A misspelt argument used to be read as nothing at all, so the call went on
//! without it and said it had worked: `place_clip` given `duration` ran the
//! clip to the end of its source, and `clip_set` given `opacity` beside
//! `scale` set the scale and lost the opacity (#1117). The price of a typo was
//! a silent wrong edit; now it is a refusal naming the argument, and the one
//! that was probably meant.
//!
//! The same holds one level down and further (#1330): a key inside an object
//! argument — `clip_set`'s `shadow`, a scene of `cut_to_voice` — is checked
//! against the `properties` the published schema lists for it, so
//! `{"shadow": {"blurr": 4}}` is refused as `shadow.blurr` rather than dropped.
//! The schema is the one source: an object whose schema lists no
//! `properties`, or says `additionalProperties` is anything but `false`, is
//! open and takes any key — whatever reads it has its own rule.

use scorsese_core::spelling::closest;
use serde_json::{Map, Value};

/// The refusal for the first object in `given`, at any depth, holding a key
/// its part of `schema` does not list, or `None` when every key is listed.
pub(super) fn refused(given: &Value, schema: &Value) -> Option<String> {
    within(given, schema, "")
}

/// [`refused`], for the value found at `at` (`""` being the arguments
/// themselves, then `shadow`, `scenes[2]`, …).
fn within(given: &Value, schema: &Value, at: &str) -> Option<String> {
    // A choice of shapes: the value is fine when any of them takes it, and
    // otherwise refused as the first one says.
    if let Some(Value::Array(branches)) = schema.get("anyOf").or_else(|| schema.get("oneOf")) {
        let said: Vec<Option<String>> = branches
            .iter()
            .map(|branch| within(given, branch, at))
            .collect();
        return match said.iter().any(Option::is_none) {
            true => None,
            false => said.into_iter().flatten().next(),
        };
    }
    match given {
        Value::Object(fields) => {
            let Some(Value::Object(properties)) = schema.get("properties") else {
                return None;
            };
            let closed = matches!(
                schema.get("additionalProperties"),
                None | Some(Value::Bool(false))
            );
            if closed && let Some(refused) = named(fields, properties, at) {
                return Some(refused);
            }
            fields.iter().find_map(|(name, value)| {
                let inner = properties.get(name)?;
                let at = match at {
                    "" => name.clone(),
                    at => format!("{at}.{name}"),
                };
                within(value, inner, &at)
            })
        }
        Value::Array(items) => {
            let inner = schema.get("items")?;
            items
                .iter()
                .enumerate()
                .find_map(|(index, item)| within(item, inner, &format!("{at}[{index}]")))
        }
        _ => None,
    }
}

/// The refusal for every key of `given` that `properties` does not list, the
/// object being the one at `at`.
fn named(given: &Map<String, Value>, properties: &Map<String, Value>, at: &str) -> Option<String> {
    let known: Vec<&str> = properties.keys().map(String::as_str).collect();
    let path = |name: &str| match at {
        "" => format!("`{name}`"),
        at => format!("`{at}.{name}`"),
    };
    let said: Vec<String> = given
        .keys()
        .map(String::as_str)
        .filter(|name| !known.contains(name))
        .map(|name| match nearest(name, &known) {
            Some(meant) => format!(
                "{} is not an argument here; did you mean {}?",
                path(name),
                path(meant)
            ),
            None => format!("{} is not an argument here.", path(name)),
        })
        .collect();
    if said.is_empty() {
        return None;
    }
    let takes = known
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let whose = match at {
        "" => "it".to_owned(),
        at => format!("`{at}`"),
    };
    Some(format!(
        "{} Nothing was done; {whose} takes {takes}.",
        said.join(" ")
    ))
}

/// The argument `name` was probably meant to be, when one stands out.
///
/// Two ways to miss: a typo close to the name (`opactiy`), by [`closest`] —
/// the rule `ANIMATABLE.did_you_mean` keeps too — or, failing that, the name
/// without its unit (`duration` for `duration_seconds`). Nothing is suggested
/// unless exactly one candidate answers: a wrong suggestion is worse than
/// none, because it is the one thing a reader acts on without checking.
fn nearest<'a>(name: &str, known: &[&'a str]) -> Option<&'a str> {
    if let Some(meant) = closest(name, known.iter().copied()) {
        return Some(meant);
    }
    let prefix = format!("{name}_");
    let extended: Vec<&str> = known
        .iter()
        .copied()
        .filter(|candidate| candidate.starts_with(&prefix))
        .collect();
    match extended.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
