//! Arguments a tool does not take, refused by name.
//!
//! A misspelt argument used to be read as nothing at all, so the call went on
//! without it and said it had worked: `place_clip` given `duration` ran the
//! clip to the end of its source, and `clip_set` given `opacity` beside
//! `scale` set the scale and lost the opacity (#1117). The price of a typo was
//! a silent wrong edit; now it is a refusal naming the argument, and the one
//! that was probably meant.

use serde_json::{Map, Value};

/// The refusal for every argument in `given` that is not one of `known`, or
/// `None` when each is.
pub(super) fn refused(given: &Map<String, Value>, known: &[&str]) -> Option<String> {
    let unknown: Vec<&str> = given
        .keys()
        .map(String::as_str)
        .filter(|name| !known.contains(name))
        .collect();
    let said: Vec<String> = unknown
        .iter()
        .map(|name| match nearest(name, known) {
            Some(meant) => format!("`{name}` is not an argument here; did you mean `{meant}`?"),
            None => format!("`{name}` is not an argument here."),
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
    Some(format!(
        "{} Nothing was done; it takes {takes}.",
        said.join(" ")
    ))
}

/// The argument `name` was probably meant to be, when one stands out.
///
/// Two ways to miss: a typo close to the name (`opactiy`), or the name without
/// its unit (`duration` for `duration_seconds`). Nothing is suggested unless
/// exactly one candidate answers — a wrong suggestion is worse than none,
/// because it is the one thing a reader acts on without checking (the rule
/// `ANIMATABLE.did_you_mean` keeps too).
fn nearest<'a>(name: &str, known: &[&'a str]) -> Option<&'a str> {
    /// Two edits: a swapped pair of letters costs exactly this.
    const NEAREST: usize = 2;

    let close: Vec<(usize, &str)> = known
        .iter()
        .map(|candidate| (strsim::levenshtein(name, candidate), *candidate))
        .filter(|(apart, candidate)| *apart <= NEAREST && apart * 2 < candidate.len())
        .collect();
    let best = close.iter().map(|(apart, _)| *apart).min();
    let closest: Vec<&str> = close
        .iter()
        .filter(|(apart, _)| Some(*apart) == best)
        .map(|(_, candidate)| *candidate)
        .collect();
    if let [only] = closest.as_slice() {
        return Some(only);
    }
    if !closest.is_empty() {
        return None;
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
mod tests {
    use super::nearest;

    const KNOWN: &[&str] = &[
        "clip",
        "scale",
        "start_seconds",
        "duration_seconds",
        "speed",
    ];

    #[test]
    fn a_typo_or_a_missing_unit_is_named() {
        assert_eq!(nearest("sclae", KNOWN), Some("scale"));
        assert_eq!(nearest("duration", KNOWN), Some("duration_seconds"));
        assert_eq!(nearest("start", KNOWN), Some("start_seconds"));
        // One letter off a three-letter name is still that name.
        assert_eq!(nearest("fot", &["fit"]), Some("fit"));
    }

    #[test]
    fn nothing_is_named_unless_one_answer_stands_out() {
        assert_eq!(nearest("opacity", KNOWN), None);
        // As close to one as to the other: no guess.
        assert_eq!(nearest("cot", &["cat", "cut"]), None);
        // Two letters off a four-letter name is a different word.
        assert_eq!(nearest("clop", &["chip"]), None);
    }
}
