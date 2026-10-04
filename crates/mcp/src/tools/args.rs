//! A tool's arguments, as one Rust type.
//!
//! Each tool declares a struct deriving `Deserialize` and `JsonSchema`, and
//! that struct is both halves of its contract: [`schema`] is what `tools/list`
//! shows a client, and [`parse`] is what the call reads back. One definition,
//! so the schema a client fills in and the parsing that reads it cannot drift
//! apart (#745). A field's doc comment *is* its `description`, which keeps the
//! rule that every argument describes itself where the argument is declared.
//!
//! **Refusals read the same from every tool.** An agent reads them and acts on
//! them, so there are two shapes and no others: *`` `clip` is required: a clip
//! id ``* for an argument left out, and *`` `start` has to be a number, not
//! "soon" ``* for one of the wrong kind. Both are built here, and only here —
//! a field type with a stricter rule than its JSON type (a path that may not be
//! blank) refuses with the second half of a sentence and this module adds the
//! name.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

/// Each required argument's name, and what it is — the end of the sentence
/// *`` `name` is required: … ``*.
pub(crate) type Required = &'static [(&'static str, &'static str)];

/// A tool's arguments: what it takes, and how to say an argument is missing.
pub(crate) trait Arguments: DeserializeOwned + JsonSchema {
    /// What each required argument is, finishing the sentence *`` `name` is
    /// required: … ``*. `project` needs no entry: every tool asks for it the
    /// same way, so it is said the same way ([`ProjectDir`]).
    const REQUIRED: Required = &[];
}

/// What a refusal of one argument says after its name.
///
/// A field type that refuses a value hands this to serde as its error, and
/// [`parse`] puts the argument's name in front — the type knows what is wrong
/// with the value and never which argument it was given as.
pub(crate) const MISSING: &str = "is required";

/// The JSON Schema of `T`, in the shape this server has always published.
///
/// `schemars` writes a few things the hand-written schemas never carried —
/// a meta-schema URI, the struct's name and doc as a title and description, a
/// Rust `format` on every number, a nullable type on every `Option`, a
/// `default` on every flag — and a
/// doc comment's line breaks. Each is taken off here, once, so that what a
/// client sees is exactly what it saw before: the type, the description, and
/// what is required.
pub(crate) fn schema<T: Arguments>() -> Value {
    let settings = SchemaSettings::draft2020_12().with(|settings| {
        settings.inline_subschemas = true;
        settings.meta_schema = None;
    });
    let mut schema = settings
        .into_generator()
        .into_root_schema_for::<T>()
        .to_value();
    if let Some(root) = schema.as_object_mut() {
        root.remove("title");
        root.remove("description");
        if let Some(Value::Object(properties)) = root.get_mut("properties") {
            properties.values_mut().for_each(tidied);
        }
    }
    schema
}

/// One property's schema with `schemars`' additions taken off, and the same
/// for every schema nested in it — an array's items, an object's properties.
fn tidied(schema: &mut Value) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    untyped_null(object);
    // A `#[serde(default)]` flag is how a field says "absent means false";
    // the schemas never published that default, and saying it now would be a
    // difference on the wire with nothing behind it.
    object.remove("default");
    if let Some(Value::String(format)) = object.remove("format")
        && format.starts_with("uint")
        && object.get("minimum") == Some(&Value::from(0))
    {
        object.remove("minimum");
    }
    if let Some(Value::String(description)) = object.get_mut("description") {
        *description = description.replace('\n', " ");
    }
    for (key, nested) in object.iter_mut() {
        match (key.as_str(), nested) {
            ("items" | "additionalProperties", nested) => tidied(nested),
            ("properties", Value::Object(properties)) => properties.values_mut().for_each(tidied),
            ("anyOf" | "oneOf" | "allOf" | "prefixItems", Value::Array(each)) => {
                each.iter_mut().for_each(tidied);
            }
            _ => {}
        }
    }
}

/// Every trace of `null` an `Option` leaves, taken back out: an argument left
/// out is how a client says nothing, and none of these schemas ever asked it
/// to send `null` instead.
///
/// Three shapes, one per kind of schema `schemars` wraps: `"type": ["string",
/// "null"]` for a plain value, a `null` among an enum's values, and `anyOf`
/// with a `{"type": "null"}` branch for an object or an array — that last one
/// folded back into the one branch left, keeping the field's description.
fn untyped_null(object: &mut Map<String, Value>) {
    if let Some(Value::Array(types)) = object.get_mut("type") {
        types.retain(|kind| kind != "null");
        if let [only] = types.as_slice() {
            let only = only.clone();
            object.insert("type".to_owned(), only);
        }
    }
    if let Some(Value::Array(values)) = object.get_mut("enum") {
        values.retain(|value| !value.is_null());
    }
    let null = serde_json::json!({ "type": "null" });
    if let Some(Value::Array(branches)) = object.get("anyOf")
        && let [kept] = branches
            .iter()
            .filter(|branch| **branch != null)
            .collect::<Vec<_>>()
            .as_slice()
        && branches.len() == 2
        && let Value::Object(kept) = (*kept).clone()
    {
        object.remove("anyOf");
        for (key, value) in kept {
            object.entry(key).or_insert(value);
        }
    }
}

/// The arguments a client sent, read as `T` — or the refusal saying which one
/// is wrong and why.
///
/// Arguments that are not an object at all are read as no arguments, so the
/// refusal names the first one missing rather than complaining about JSON.
/// Arguments the type does not know are ignored, as they always were.
pub(crate) fn parse<T: Arguments>(arguments: &Value) -> Result<T, String> {
    let empty = Value::Object(Map::new());
    let arguments = if arguments.is_object() {
        arguments
    } else {
        &empty
    };
    serde_path_to_error::deserialize(arguments).map_err(|error| {
        let field = error.path().to_string();
        let said = error.into_inner().to_string();
        refusal::<T>(arguments, &field, &said)
    })
}

/// The sentence for one refused argument.
fn refusal<T: Arguments>(arguments: &Value, field: &str, said: &str) -> String {
    if let Some(name) = said
        .strip_prefix("missing field `")
        .and_then(|rest| rest.strip_suffix('`'))
    {
        return required::<T>(name);
    }
    if said == MISSING {
        return required::<T>(field);
    }
    let what = said
        .rsplit_once(", expected ")
        .map_or(said, |(_, expected)| expected);
    let what = kind(what);
    match arguments.get(field) {
        Some(given) => format!("`{field}` has to be {what}, not {given}"),
        None => format!("`{field}` has to be {what}"),
    }
}

/// A text argument with surrounding blanks trimmed, and nothing left counting
/// as absent — a blank filter or id is somebody not saying one, and acting on
/// it would narrow a search to nothing or look for an id nobody has.
pub(crate) fn given(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// A path argument, resolved against the project directory unless it is
/// already absolute; `None` when it was left out.
///
/// Relative-to-the-project is the rule every path in this surface obeys: the
/// server's working directory belongs to whoever launched it, so a relative
/// path resolved against it lands somewhere the caller did not name and cannot
/// read back (#496). An absolute path is still honoured, because a measured
/// render or a partial bake is as likely to sit outside the project as in it.
/// Whether a *write* may leave the project is a separate question this does
/// not answer.
pub(crate) fn under(
    dir: &Path,
    given: Option<&str>,
    field: &str,
) -> Result<Option<PathBuf>, String> {
    given.map(|given| path(dir, given, field)).transpose()
}

/// The same, for a path argument that was given: blank is refused rather than
/// read as the project directory itself.
pub(crate) fn path(dir: &Path, given: &str, field: &str) -> Result<PathBuf, String> {
    if given.trim().is_empty() {
        return Err(format!("`{field}` is empty — give a path or leave it out"));
    }
    let path = PathBuf::from(given);
    Ok(if path.is_absolute() {
        path
    } else {
        dir.join(path)
    })
}

/// *`` `name` is required: … ``*, with the what when the tool says one.
fn required<T: Arguments>(name: &str) -> String {
    let what = T::REQUIRED
        .iter()
        .chain(&[("project", PROJECT)])
        .find(|(field, _)| *field == name)
        .map(|(_, what)| *what);
    match what {
        Some(what) => format!("`{name}` {MISSING}: {what}"),
        None => format!("`{name}` {MISSING}"),
    }
}

/// serde's name for what it expected, as a sentence says it.
fn kind(expected: &str) -> &str {
    match expected {
        "u8" | "u16" | "u32" | "u64" | "usize" => "a whole number",
        "i8" | "i16" | "i32" | "i64" | "isize" => "a whole number",
        "f32" | "f64" => "a number",
        "a boolean" => "true or false",
        other => other,
    }
}

/// What the project argument is, when it is missing.
const PROJECT: &str = "the path of the *.scor directory";

/// The project directory a call works on.
///
/// Every tool takes one, and it is required rather than defaulted to the
/// working directory: a server started by a client has no meaningful working
/// directory, and guessing one is how you edit the wrong film. A blank one is
/// refused as missing, for the same reason.
///
/// Its own type so that its schema is written once: a field of this type needs
/// no doc comment, and every tool spells and describes it the same way.
#[derive(Debug, Clone)]
pub(crate) struct ProjectDir(PathBuf);

impl ProjectDir {
    /// The directory, as given.
    pub(crate) fn dir(&self) -> &Path {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProjectDir {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let given = String::deserialize(deserializer)?;
        if given.trim().is_empty() {
            return Err(D::Error::custom(MISSING));
        }
        Ok(Self(PathBuf::from(given)))
    }
}

impl JsonSchema for ProjectDir {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "ProjectDir".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Path to the *.scor project directory to work on."
        })
    }
}

/// The arguments of a tool that takes nothing but the project.
#[derive(Deserialize, JsonSchema)]
pub(crate) struct ProjectOnly {
    // Undocumented on purpose: a doc comment here would replace the one
    // `ProjectDir` gives every tool.
    pub(crate) project: ProjectDir,
}

impl Arguments for ProjectOnly {}

/// A required text argument that may not be blank: an id, a name, a query.
///
/// Blank is refused as missing, with the tool's own word for what it is
/// ([`Arguments::REQUIRED`]), because an empty id is somebody not saying one.
/// What is kept is trimmed. An *optional* text argument is an
/// `Option<String>` read through [`given`] instead, where blank means absent.
#[derive(Debug, Clone)]
pub(crate) struct Name(String);

impl Name {
    /// The text, trimmed.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let given = String::deserialize(deserializer)?;
        match given.trim() {
            "" => Err(D::Error::custom(MISSING)),
            trimmed => Ok(Self(trimmed.to_owned())),
        }
    }
}

impl JsonSchema for Name {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "Name".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

#[cfg(test)]
mod tests;
