//! The project document: `project.json` and the operations that read it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetId};
use crate::note::Noted;
use crate::path::ProjectPath;
use crate::time::Fps;
use crate::timeline::{Clip, Track};
use crate::validate::ValidationErrors;

/// The schema version this build reads and writes.
///
/// Bumping it is `architecture` work and requires a migration note: this
/// format is the contract between the CLI, the MCP server, the GUI, and every
/// project already saved on someone's disk.
pub const SCHEMA_VERSION: u32 = 14;

/// The document's file name inside a `*.scor/` project directory.
pub const PROJECT_FILE_NAME: &str = "project.json";

/// Imported media, copied in so the project stays self-contained.
pub const ASSETS_DIR: &str = "assets";

/// Provider and synthesis output, content-addressed by the hash of the brief
/// that produced it.
pub const GENERATED_DIR: &str = "generated";

/// Authored documents that generate assets — synthesis recipes today.
///
/// Its own directory because the other three each say something a recipe is
/// not: it is neither imported media nor output, and it is not rebuildable —
/// deleting one loses work. Named for the role rather than for audio, so a
/// procedural texture recipe would have somewhere to go without another
/// format change.
pub const RECIPES_DIR: &str = "recipes";

/// Rebuildable scratch. Gitignored, and safe to delete at any time.
pub const CACHE_DIR: &str = "cache";

/// A whole project: the assets it knows about and where they sit in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Which format this document is written in. This build reads exactly
    /// [`SCHEMA_VERSION`] and refuses the rest, rather than guessing what an
    /// older or newer writer meant by a field.
    pub schema_version: u32,
    /// What a human calls this edit. Cosmetic, and unrelated to the `*.scor/`
    /// directory's own name.
    pub name: String,
    /// The grid this edit is authored against. Every clip and keyframe time
    /// in the document is a frame count on it.
    ///
    /// Required, with no default: a missing framerate would leave every time
    /// in the file meaning something other than what its author intended, and
    /// guessing 30 is a worse failure than refusing to load.
    pub timeline_fps: Fps,
    /// The document this edit is being cut from, as a project-relative path —
    /// by convention `script.md` at the root.
    ///
    /// **A file, not a string.** The same reasoning as a synthesis
    /// [`Asset::recipe`]: such a document is long — a real one measured 30 KB
    /// against a `project.json` of 34 KB — and inlining it would very nearly
    /// double the document while burying the timeline under prose, in the one
    /// file an agent opens to learn the edit. A file also gets a readable diff
    /// and edits from any tool. It lives inside the project directory like
    /// every other path, so it still survives `scp -r`.
    ///
    /// **Never parsed.** No schema, no sections, no front matter, no
    /// convention this tool enforces — the moment something here extracts
    /// meaning from it, it becomes a format with rules and stops being the
    /// place you write freely. Markdown is a convention and not a requirement;
    /// nothing reads the extension.
    ///
    /// **Never rendered**, exactly as [`crate::Track::note`] is not.
    ///
    /// A `script` naming a file that is not there is a **warning**, not a
    /// failure, for the same reason a missing generated file is: a project that
    /// has lost its brief should still render.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ProjectPath>,
    /// Every asset the project knows about, keyed by `id` within the entries.
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// The tracks, in compositing order: the first video track is the bottom
    /// layer. Audio tracks are unordered among themselves.
    #[serde(default)]
    pub tracks: Vec<Track>,
}

impl Project {
    /// An empty project on the given grid, at the current schema version.
    pub fn new(name: impl Into<String>, timeline_fps: Fps) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            timeline_fps,
            script: None,
            assets: Vec::new(),
            tracks: Vec::new(),
        }
    }

    /// Creates a `*.scor/` directory: the sub-directories and a
    /// `project.json` for an empty project on the given grid. Refuses to
    /// overwrite one that is already there.
    ///
    /// **The name is optional, and the fallback lives here** rather than in
    /// whichever client asked. `teaser.scor` holding a project called `teaser`
    /// is what anyone omitting it meant, and every client that starts a project
    /// — the command line, the MCP server — has to mean the same thing by it,
    /// or two projects made the same way end up named differently.
    pub fn create(
        project_dir: &Path,
        name: Option<&str>,
        timeline_fps: Fps,
    ) -> Result<Self, SaveError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        if file.exists() {
            return Err(SaveError::AlreadyAProject {
                path: project_dir.to_path_buf(),
            });
        }
        for directory in [ASSETS_DIR, GENERATED_DIR, RECIPES_DIR, CACHE_DIR] {
            let path = project_dir.join(directory);
            fs::create_dir_all(&path).map_err(|source| SaveError::Io { path, source })?;
        }
        let project = Self::new(name_for(project_dir, name), timeline_fps);
        project.save(project_dir)?;
        Ok(project)
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

    /// Every note in the document, in reading order: the assets table first,
    /// then each track followed by the clips on it.
    ///
    /// One pass over the whole document rather than three lists, because the
    /// question anyone asks is *what does this project say about itself?* and
    /// the answer is not sorted by which table a note happens to live in.
    pub fn notes(&self) -> impl Iterator<Item = Noted<'_>> {
        let assets = self.assets.iter().filter_map(Noted::on_asset);
        let tracks = self.tracks.iter().flat_map(|track| {
            Noted::on_track(track)
                .into_iter()
                .chain(track.clips.iter().filter_map(Noted::on_clip))
        });
        assets.chain(tracks)
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
    ///
    /// The write is atomic — see [`crate::write`] — so a reader arriving
    /// mid-save gets the previous document whole, never half of this one.
    pub fn save(&self, project_dir: &Path) -> Result<(), SaveError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        let json = self.to_json()?;
        crate::write::atomically(&file, json).map_err(|source| SaveError::Io { path: file, source })
    }
}

/// What a new project is called: what was asked for, or the directory's own
/// stem with its `.scor` extension dropped.
///
/// `untitled` is the last resort, for a directory with no name of its own to
/// borrow — `.` and `/` are the ones that reach it. A project with an awkward
/// name is a field anyone can edit; a refusal here would be creation failing
/// over something cosmetic.
fn name_for(project_dir: &Path, asked: Option<&str>) -> String {
    asked
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map_or_else(
            || {
                project_dir
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("untitled")
                    .to_owned()
            },
            str::to_owned,
        )
}

/// Reads `schema_version` alone, tolerating everything else in the document.
#[derive(Deserialize)]
struct VersionPeek {
    schema_version: u32,
}

/// Why a project could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The file could not be read — usually no `project.json` where one was
    /// expected, so the directory is not a project.
    #[error("reading {path}: {source}")]
    Io {
        /// The `project.json` that could not be read.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// Not valid JSON, or JSON that does not fit the schema — an unknown field
    /// lands here, since a typo like `"trackz"` fails the load rather than
    /// being dropped. Carries the line it is on.
    #[error("parsing project.json: {0}")]
    Parse(#[source] serde_json::Error),
    /// A document from another build. Read before anything else, so the
    /// advice is "upgrade scorsese" rather than a complaint about a field.
    #[error("project.json is schema_version {found}, but this build of scorsese reads {supported}")]
    SchemaVersion {
        /// The version the document declares.
        found: u32,
        /// The one and only version this build reads.
        supported: u32,
    },
    /// The document parsed but does not describe a coherent project. Every
    /// problem at once, never just the first.
    #[error(transparent)]
    Invalid(#[from] ValidationErrors),
}

/// Why a project could not be saved.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// The in-memory project could not be turned into JSON. A non-finite
    /// keyframe value is the realistic way to get here.
    #[error("serialising project: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The write itself failed: no such directory, no permission, no room.
    #[error("writing {}: {source}", path.display())]
    Io {
        /// The file or directory that could not be written.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// Creating a project where one already lives. Refused rather than
    /// overwritten — clobbering an edit is not undoable.
    #[error("{} is already a scorsese project", path.display())]
    AlreadyAProject {
        /// The directory that already holds a `project.json`.
        path: PathBuf,
    },
}
