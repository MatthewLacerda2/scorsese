//! The project document: `project.json` and the operations that read it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetId};
use crate::timeline::{Clip, Track};
use crate::validate::ValidationErrors;

/// The schema version this build reads and writes.
///
/// Bumping it is `architecture` work and requires a migration note: this
/// format is the contract between the CLI, the MCP server, the GUI, and every
/// project already saved on someone's disk.
pub const SCHEMA_VERSION: u32 = 1;

/// The document's file name inside a `*.scor/` project directory.
pub const PROJECT_FILE_NAME: &str = "project.json";

/// A whole project: the assets it knows about and where they sit in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    /// Every asset the project knows about, keyed by `id` within the entries.
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

impl Project {
    /// An empty project at the current schema version.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            assets: Vec::new(),
            tracks: Vec::new(),
        }
    }

    /// Looks an asset up by id.
    pub fn asset(&self, id: &AssetId) -> Option<&Asset> {
        self.assets.iter().find(|asset| &asset.id == id)
    }

    /// Every clip in the project, paired with the track holding it.
    pub fn clips(&self) -> impl Iterator<Item = (&Track, &Clip)> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter().map(move |clip| (track, clip)))
    }

    /// Assets GO would spend money on: the sketch and stale ones.
    pub fn pending_generation(&self) -> impl Iterator<Item = &Asset> {
        self.assets.iter().filter(|asset| asset.needs_generation())
    }

    /// Parses a document **without** validating it. Structural problems
    /// (bad JSON, unknown fields, wrong schema version) still fail here;
    /// semantic ones are [`Project::validate`]'s job.
    pub fn from_json(json: &str) -> Result<Self, LoadError> {
        // The version is read on its own first. A document from a future
        // scorsese will contain fields this build rejects, and "upgrade
        // scorsese" is the useful thing to say about it — not "unknown field".
        let found = serde_json::from_str::<VersionPeek>(json).map_err(LoadError::Parse)?;
        if found.schema_version != SCHEMA_VERSION {
            return Err(LoadError::SchemaVersion {
                found: found.schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        serde_json::from_str(json).map_err(LoadError::Parse)
    }

    /// Serialises the document as it is written to disk.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    /// Reads and validates `project.json` from a `*.scor/` directory. This is
    /// the strict path: an invalid project does not load.
    pub fn load(project_dir: &Path) -> Result<Self, LoadError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        let json = fs::read_to_string(&file).map_err(|source| LoadError::Io {
            path: file.clone(),
            source,
        })?;
        let project = Self::from_json(&json)?;
        project.validate()?;
        Ok(project)
    }

    /// Writes `project.json` into a `*.scor/` directory.
    ///
    /// Does **not** validate: an editor mid-edit is allowed to save work that
    /// is temporarily incoherent. Validate before rendering, not before
    /// saving.
    pub fn save(&self, project_dir: &Path) -> Result<(), SaveError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        let json = self.to_json()?;
        fs::write(&file, json).map_err(|source| SaveError::Io { path: file, source })
    }
}

/// Reads `schema_version` alone, tolerating everything else in the document.
#[derive(Deserialize)]
struct VersionPeek {
    schema_version: u32,
}

/// Why a project could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing project.json: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("project.json is schema_version {found}, but this build of scorsese reads {supported}")]
    SchemaVersion { found: u32, supported: u32 },
    #[error(transparent)]
    Invalid(#[from] ValidationErrors),
}

/// Why a project could not be saved.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("serialising project: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("writing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
