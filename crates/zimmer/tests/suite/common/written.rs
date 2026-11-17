//! What a document writes back: the keys a round trip adds, held to none.
//!
//! The address of a bake hashes the recipe's **bytes**, and tools re-serialise
//! a recipe whenever they write one. So an optional field that writes its
//! default into every document changes every recipe's bytes, and every cached
//! bake misses, without a sample moving (`crates/zimmer/CLAUDE.md`). Each such
//! field takes `skip_serializing_if`, and [`drift`] holds every one of them
//! at once — including the next one, which is the point.
//!
//! Shared by path with `crates/providers/tests/suite/documented`, which holds the
//! worked recipes in `docs/recipes.md` to the same.

use std::collections::BTreeSet;

use serde_json::Value;

/// Every key the round trip from `written` to `rewritten` added or dropped,
/// as a failure message; `what` names the
/// document. `None` when the shape held.
///
/// `root` names what the document's top level is, so a bare patch's fields
/// read as `patch.…` the way they do inside a song or a recipe.
pub(crate) fn drift(what: &str, root: &str, written: &str, rewritten: &str) -> Option<String> {
    let before = keys(root, written);
    let after = keys(root, rewritten);
    let added: Vec<_> = after.difference(&before).collect();
    let dropped: Vec<_> = before.difference(&after).collect();
    (!added.is_empty() || !dropped.is_empty()).then(|| {
        format!(
            "{what} changed shape on the way back out — added {added:?}, dropped \
         {dropped:?}. An optional field that writes its default changes every \
         recipe's bytes and so every bake's address: give it \
         `skip_serializing_if` at its default (crates/zimmer/CLAUDE.md)."
        )
    })
}

/// Every key in a JSON document, as a path from `root`, array indices erased.
fn keys(root: &str, json: &str) -> BTreeSet<String> {
    let value: Value = serde_json::from_str(json).expect("the document is JSON");
    let mut found = BTreeSet::new();
    walk(&value, root.to_owned(), &mut found);
    found
}

fn walk(value: &Value, at: String, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            for (key, inner) in fields {
                let path = format!("{at}.{key}");
                found.insert(path.clone());
                walk(inner, path, found);
            }
        }
        Value::Array(items) => {
            for inner in items {
                walk(inner, format!("{at}[]"), found);
            }
        }
        _ => {}
    }
}
