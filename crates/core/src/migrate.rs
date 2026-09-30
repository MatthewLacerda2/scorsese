//! Bringing a document written by an older build forward to this one.
//!
//! **Every `schema_version` bump carries a migration** (`CLAUDE.md`, *A schema
//! bump ships with a migration*): a step over the JSON document that rewrites
//! a `vN-1` document into `vN`'s shape. Steps chain, so a document several
//! versions behind walks forward one step at a time, and each step only ever
//! has to understand the two versions either side of it.
//!
//! **Over the JSON, not over [`Project`].** A step exists precisely because the
//! old document no longer deserialises into this build's types — a renamed
//! field is an unknown field to `deny_unknown_fields` — so the only thing both
//! ends of a step can be expressed in is the document itself.
//!
//! **One implementation, two callers.** The web server runs [`parse`] over
//! every stored project when it starts on a new build, and `scorsese migrate`
//! runs [`folder`] over a local `.scor` directory. Neither has a step of its
//! own; both would be the same code getting the same answer twice.
//!
//! **Migrating is not reading in place.** [`Project::from_json`] and
//! [`Project::load`] still refuse any version that is not this build's: a
//! document meets this module once, is rewritten at the current version, and
//! is read by the strict path from then on. That keeps "this build understands
//! v35" a statement about one version rather than about a range.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::baseline::Baseline;
use crate::project::{LoadError, PROJECT_FILE_NAME, Project, SCHEMA_VERSION, SaveError};

/// The oldest `schema_version` this build can bring forward.
///
/// Documents older than this predate the rule: v33 was the format when
/// projects started living in other people's accounts (#534), and nothing
/// older was ever stored anywhere but the maintainer's own disk. It moves
/// back never, and forward only if the user decides a version is too old to
/// carry — which is a question for them, not a thing to do in passing.
pub const OLDEST_MIGRATABLE: u32 = 33;

/// One step: rewrites a document at `from` into `from + 1`'s shape.
///
/// The step changes the content; [`walk`] sets `schema_version` afterwards,
/// so no step can forget to, and none can set it to the wrong number.
pub(crate) struct Step {
    /// The version this step reads.
    pub(crate) from: u32,
    /// The rewrite. An `Err` says why this particular document cannot be
    /// carried forward — its old meaning having no new equivalent.
    pub(crate) apply: fn(&mut Value) -> Result<(), String>,
}

/// Every step this build knows, oldest first — one for each version from
/// [`OLDEST_MIGRATABLE`] up to the one before [`SCHEMA_VERSION`].
///
/// The test beside this module fails the day [`SCHEMA_VERSION`] moves without
/// a step being added here.
pub(crate) const STEPS: &[Step] = &[
    Step {
        from: 33,
        apply: groups_arrive,
    },
    Step {
        from: 34,
        apply: easings_arrive,
    },
];

/// v33 → v34: the `group` asset kind (#586).
///
/// **Nothing to rewrite, and that is the whole of the step.** The version
/// added a kind — an asset holding tracks of its own — and a `group` block
/// only that kind may carry, and it changed the meaning of nothing a v33
/// document can say: every v33 asset, clip and field reads the same at v34. So
/// the document passes through untouched and only its `schema_version` moves,
/// which [`walk`] does after every step.
///
/// It is still a step rather than an absence, because the chain has to be
/// able to walk *from* 33: a missing step is what [`MigrateError::TooOld`]
/// reports, and a v33 project in somebody's account is exactly the document
/// this rule exists to carry forward.
fn groups_arrive(_: &mut Value) -> Result<(), String> {
    Ok(())
}

/// v34 → v35: the overshooting easings and `cubic_bezier` (#587).
///
/// Nothing to rewrite, for [`groups_arrive`]'s reason: the version added
/// values an `easing` may take — `back_in`, `back_out`, `back_in_out`,
/// `spring` and `{ "cubic_bezier": [..] }` — and every easing a v34 document
/// can hold still names the same curve with the same arithmetic.
fn easings_arrive(_: &mut Value) -> Result<(), String> {
    Ok(())
}

/// Why a document could not be brought forward.
#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    /// Not a JSON object with a whole-number `schema_version`.
    #[error("the document has no schema_version to migrate from")]
    Unversioned,
    /// Written by a newer build. Nothing goes backwards.
    #[error(
        "the document is schema_version {found}, newer than this build's {supported}: \
         upgrade scorsese rather than migrating"
    )]
    Newer {
        /// The version the document declares.
        found: u32,
        /// The version this build writes.
        supported: u32,
    },
    /// Older than anything this build can carry forward.
    #[error(
        "the document is schema_version {found}, and this build migrates nothing older \
         than {OLDEST_MIGRATABLE}"
    )]
    TooOld {
        /// The version the document declares.
        found: u32,
    },
    /// A step refused this document.
    #[error("migrating from schema_version {from}: {reason}")]
    Failed {
        /// The version the refusing step reads.
        from: u32,
        /// What it said.
        reason: String,
    },
    /// The chain ran, and what it produced is still not a document this build
    /// reads — a bug in a step, since the step's own test should have caught it.
    #[error("the migrated document does not load: {0}")]
    Unloadable(#[source] LoadError),
    /// Not JSON at all.
    #[error("parsing project.json: {0}")]
    Parse(#[source] serde_json::Error),
    /// The folder's `project.json` could not be read.
    #[error("reading {}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// The migrated document could not be written back.
    #[error(transparent)]
    Save(#[from] SaveError),
}

/// Brings `document` forward to [`SCHEMA_VERSION`] in place.
///
/// `Ok(None)` when it was already current and nothing was touched;
/// `Ok(Some(v))` when it was at `v` and has been rewritten.
pub fn document(document: &mut Value) -> Result<Option<u32>, MigrateError> {
    walk(document, STEPS, SCHEMA_VERSION)
}

/// Reads a document of any version this build can carry, as a [`Project`] at
/// this one — and the version it started at, when that was not this one.
///
/// Parsed, not validated: exactly what [`Project::from_json`] promises, one
/// step earlier.
pub fn parse(json: &str) -> Result<(Project, Option<u32>), MigrateError> {
    let mut value: Value = serde_json::from_str(json).map_err(MigrateError::Parse)?;
    let from = document(&mut value)?;
    let project = match from {
        // Current already: parsed from the original text, so an error points
        // at the line the author wrote rather than a re-serialisation of it.
        None => Project::from_json(json),
        Some(_) => serde_json::from_value(value).map_err(LoadError::Parse),
    }
    .map_err(MigrateError::Unloadable)?;
    Ok((project, from))
}

/// Rewrites a `.scor` folder's `project.json` at this build's version.
///
/// A folder already current is not touched at all. The write is the ordinary
/// [`Project::save`], so it is atomic and refuses to land on a document that
/// changed while this ran.
pub fn folder(project_dir: &Path) -> Result<Option<u32>, MigrateError> {
    let file = project_dir.join(PROJECT_FILE_NAME);
    let bytes = std::fs::read(&file).map_err(|source| MigrateError::Io { path: file, source })?;
    let json = String::from_utf8_lossy(&bytes);
    let (mut project, from) = parse(&json)?;
    if from.is_some() {
        project.baseline = Baseline::of(&bytes);
        project.save(project_dir)?;
    }
    Ok(from)
}

/// The chain itself, over any list of steps — so a test can run it over a
/// chain that is not empty before the real one is.
pub(crate) fn walk(
    document: &mut Value,
    steps: &[Step],
    target: u32,
) -> Result<Option<u32>, MigrateError> {
    let start = version_of(document)?;
    if start > target {
        return Err(MigrateError::Newer {
            found: start,
            supported: target,
        });
    }
    if start == target {
        return Ok(None);
    }
    let mut at = start;
    while at < target {
        let step = steps
            .iter()
            .find(|step| step.from == at)
            .ok_or(MigrateError::TooOld { found: start })?;
        (step.apply)(document).map_err(|reason| MigrateError::Failed { from: at, reason })?;
        at += 1;
        document["schema_version"] = Value::from(at);
    }
    Ok(Some(start))
}

/// The document's `schema_version`, if it has a sensible one.
fn version_of(document: &Value) -> Result<u32, MigrateError> {
    document
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(MigrateError::Unversioned)
}

#[cfg(test)]
mod tests {
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

    /// The real step, held to the rule every step is held to: a document at
    /// the old version, migrated, loads and validates at this one. A v33
    /// document with something of every common shape in it — an imported
    /// shot, a title, a colour, keyframes — is the realistic case.
    #[test]
    fn a_v33_document_walks_to_this_version_and_validates() {
        let document = json!({
            "schema_version": 33,
            "name": "Before groups",
            "timeline_fps": { "num": 30, "den": 1 },
            "assets": [
                { "id": "bed", "kind": "color", "color": "#101820" },
                { "id": "title", "kind": "text", "text": "Hello" }
            ],
            "tracks": [{ "id": "v1", "kind": "video", "clips": [
                { "id": "c-bed", "asset": "bed", "start": 0, "duration": 30 }
            ]}, { "id": "v2", "kind": "video", "clips": [
                { "id": "c-title", "asset": "title", "start": 0, "duration": 30,
                  "keyframes": [{ "property": "opacity", "keyframes": [
                      { "t": 0, "value": 0.0 }, { "t": 10, "value": 1.0 }] }] }
            ]}]
        });
        let (project, from) = parse(&document.to_string()).expect("a v33 document migrates");
        assert_eq!(from, Some(33));
        assert_eq!(project.schema_version, SCHEMA_VERSION);
        project
            .validate()
            .expect("and what comes out is a valid document");
        assert_eq!(project.tracks.len(), 2, "nothing was dropped on the way");
    }

    /// v34 → v35 held to the same rule, over the part of a document the
    /// version touched: every easing a v34 keyframe could name still names the
    /// same curve after the walk.
    #[test]
    fn a_v34_documents_easings_read_the_same_at_this_version() {
        use crate::Easing;
        let words = ["linear", "ease_in", "ease_out", "ease_in_out", "hold"];
        let keyframes: Vec<Value> = (0u64..)
            .zip(words)
            .map(|(t, easing)| json!({ "t": t * 10, "value": 0.5, "easing": easing }))
            .collect();
        let document = json!({
            "schema_version": 34,
            "name": "Before overshoot",
            "timeline_fps": { "num": 30, "den": 1 },
            "assets": [{ "id": "title", "kind": "text", "text": "Hello" }],
            "tracks": [{ "id": "v1", "kind": "video", "clips": [
                { "id": "c-title", "asset": "title", "start": 0, "duration": 60,
                  "keyframes": [{ "property": "opacity", "keyframes": keyframes }] }
            ]}]
        });
        let (project, from) = parse(&document.to_string()).expect("a v34 document migrates");
        assert_eq!(from, Some(34));
        project.validate().expect("and it validates");
        let read: Vec<Easing> = project.tracks[0].clips[0].keyframes[0]
            .keyframes
            .iter()
            .map(|keyframe| keyframe.easing)
            .collect();
        let expected = [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::Hold,
        ];
        assert_eq!(read, expected);
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
}
