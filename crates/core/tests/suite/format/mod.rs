//! The format fingerprint (#1215): `project.json`'s shape at this
//! `SCHEMA_VERSION`, recorded beside the tests, so a format change that comes
//! without a version bump fails here.
//!
//! What is recorded is [`document::project`] — one project with every field
//! set — written out, and [`variants::table`], every enum variant as written.
//! Both are built so that a field or a variant added to the format does not
//! compile until it is in them; [`fields`] checks that nothing is hidden by
//! being skipped when written. Any change to how either serialises is a
//! format change, and `recorded.json` names the version it was recorded at.
//!
//! Recording is part of a bump (`crates/core/CLAUDE.md`) and refuses to
//! overwrite a record of this same version: `RECORD_FORMAT=1 cargo test -p
//! scorsese-core --test suite -- format::`.

#[macro_use]
mod variants;
mod clip;
mod document;
mod fields;
mod stock;
mod units;

use std::collections::BTreeSet;
use std::path::PathBuf;

use scorsese_core::{Project, SCHEMA_VERSION};
use serde_json::{Value, json};

fn recorded_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/suite/format/recorded.json")
}

/// The format as this build writes it.
fn written() -> Value {
    let project = serde_json::to_value(document::project()).expect("serialise");
    json!({ "project": project, "variants": variants::table() })
}

fn recorded() -> Value {
    let text = std::fs::read_to_string(recorded_path()).expect("recorded.json is beside this test");
    serde_json::from_str(&text).expect("recorded.json is JSON")
}

fn version_of(format: &Value) -> Option<u64> {
    format.pointer("/project/schema_version")?.as_u64()
}

/// Every leaf of `value` as `path = value`, so a mismatch can say where.
fn leaves(value: &Value, at: &str, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => map
            .iter()
            .for_each(|(k, v)| leaves(v, &format!("{at}/{k}"), into)),
        Value::Array(items) => {
            items
                .iter()
                .enumerate()
                .for_each(|(i, v)| leaves(v, &format!("{at}/{i}"), into));
        }
        leaf => drop(into.insert(format!("{at} = {leaf}"))),
    }
}

fn differences(recorded: &Value, written: &Value) -> String {
    let (mut was, mut now) = (BTreeSet::new(), BTreeSet::new());
    leaves(recorded, "", &mut was);
    leaves(written, "", &mut now);
    let gone = was.difference(&now).map(|leaf| format!("  - {leaf}"));
    let new = now.difference(&was).map(|leaf| format!("  + {leaf}"));
    gone.chain(new).take(40).collect::<Vec<_>>().join("\n")
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "RECORD_FORMAT is a test's recording switch, not a secret"
)]
fn the_format_is_the_one_recorded_for_this_schema_version() {
    let written = written();
    if std::env::var_os("RECORD_FORMAT").is_some() {
        let old = recorded_path().exists().then(recorded);
        if let Some(old) = old.filter(|old| version_of(old) == Some(SCHEMA_VERSION.into())) {
            assert_eq!(
                old,
                written,
                "refusing to record: the format changed at schema version {SCHEMA_VERSION}, which \
                 is already recorded. Bump SCHEMA_VERSION with a migration step first.\n{}",
                differences(&old, &written),
            );
        }
        let text = serde_json::to_string_pretty(&written).expect("serialise") + "\n";
        std::fs::write(recorded_path(), text).expect("write recorded.json");
        return;
    }
    let recorded = recorded();
    assert!(
        recorded == written,
        "project.json's format changed (recorded at schema version {}, this build is \
         {SCHEMA_VERSION}). A format change is `architecture` work: bump SCHEMA_VERSION, add \
         its migration step, then record the new format with `RECORD_FORMAT=1 cargo test -p \
         scorsese-core --test suite -- format::` (crates/core/CLAUDE.md, *A schema bump ships with a \
         migration*). Only if this test's own values changed, edit recorded.json to match.\n{}",
        version_of(&recorded).map_or("unknown".to_owned(), |v| v.to_string()),
        differences(&recorded, &written),
    );
}

#[test]
fn the_recorded_document_reads_back_as_the_one_built() {
    let recorded = recorded();
    let text = serde_json::to_string(&recorded["project"]).expect("serialise");
    let read = Project::from_json(&text).expect("the recorded document loads");
    assert_eq!(read, document::project());
}
