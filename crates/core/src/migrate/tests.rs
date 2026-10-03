//! The steps held to their rule — a document at the old version, migrated,
//! loads and validates at this one — and the walk that chains them.

use serde_json::json;

use super::*;

#[test]
fn every_version_since_the_oldest_has_exactly_one_step() {
    // The rule, held: bump SCHEMA_VERSION without a step and this fails.
    let froms: Vec<u32> = STEPS.iter().map(|step| step.from).collect();
    let expected: Vec<u32> = (OLDEST_MIGRATABLE..SCHEMA_VERSION).collect();
    assert_eq!(froms, expected, "one step per version, oldest first");
}

fn mark(document: &mut Value, marker: &str) {
    let trail = document["trail"].as_array_mut().expect("a trail");
    trail.push(Value::from(marker));
}

const CHAIN: &[Step] = &[
    Step {
        from: 1,
        apply: |document| {
            mark(document, "1→2");
            Ok(())
        },
    },
    Step {
        from: 2,
        apply: |document| {
            mark(document, "2→3");
            Ok(())
        },
    },
];

#[test]
fn the_chain_runs_every_step_in_order_and_sets_the_version() {
    let mut document = json!({ "schema_version": 1, "trail": [] });
    assert_eq!(walk(&mut document, CHAIN, 3).unwrap(), Some(1));
    assert_eq!(
        document,
        json!({ "schema_version": 3, "trail": ["1→2", "2→3"] })
    );

    let mut halfway = json!({ "schema_version": 2, "trail": [] });
    assert_eq!(walk(&mut halfway, CHAIN, 3).unwrap(), Some(2));
    assert_eq!(halfway["trail"], json!(["2→3"]));
}

#[test]
fn a_current_document_is_left_exactly_as_it_was() {
    let mut document = json!({ "schema_version": 3, "trail": [] });
    assert_eq!(walk(&mut document, CHAIN, 3).unwrap(), None);
    assert_eq!(document["trail"], json!([]));
}

#[test]
fn nothing_goes_backwards_and_nothing_is_guessed() {
    let mut newer = json!({ "schema_version": 4, "trail": [] });
    assert!(matches!(
        walk(&mut newer, CHAIN, 3),
        Err(MigrateError::Newer {
            found: 4,
            supported: 3
        })
    ));
    let mut ancient = json!({ "schema_version": 0, "trail": [] });
    assert!(matches!(
        walk(&mut ancient, CHAIN, 3),
        Err(MigrateError::TooOld { found: 0 })
    ));
    let mut unversioned = json!({ "name": "x" });
    assert!(matches!(
        walk(&mut unversioned, CHAIN, 3),
        Err(MigrateError::Unversioned)
    ));
}

#[test]
fn a_refusing_step_names_itself() {
    const REFUSES: &[Step] = &[Step {
        from: 1,
        apply: |_| Err("no new equivalent".to_owned()),
    }];
    let mut document = json!({ "schema_version": 1 });
    let error = walk(&mut document, REFUSES, 2).unwrap_err();
    assert!(
        matches!(error, MigrateError::Failed { from: 1, .. }),
        "{error}"
    );
}
