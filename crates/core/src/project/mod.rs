//! The project document: `project.json` and the operations that read it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetId};
use crate::baseline::Baseline;
use crate::frame::Ratio;
use crate::note::Noted;
use crate::path::ProjectPath;
use crate::time::Fps;
use crate::timeline::Track;
use crate::validate::ValidationErrors;

mod lanes;
mod vacant;
mod walk;

use vacant::vacant;

/// The schema version this build reads and writes.
///
/// Bumping it is `architecture` work and ships with a migration from the
/// previous version (`CLAUDE.md`, *A schema bump ships with a migration*):
/// this format is the contract between the CLI, the MCP server, the GUI, the
/// web app, and every project already stored — on a disk or in its database.
pub const SCHEMA_VERSION: u32 = 55;

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

/// Authored web pages, which `html` assets play as moving pictures.
///
/// Beside [`RECIPES_DIR`] and for the same reason: a page is a document
/// somebody wrote, not imported media and not output, and it is not
/// rebuildable — deleting one loses work. A page may reference other files in
/// the project (`../assets/photo.png`), never anything outside it.
pub const PAGES_DIR: &str = "pages";

/// Rebuildable scratch. Gitignored, and safe to delete at any time.
pub const CACHE_DIR: &str = "cache";

/// Every folder [`Project::create`] lays out beside [`PROJECT_FILE_NAME`], in
/// the order a reply lists them.
pub const PROJECT_DIRS: [&str; 5] = [ASSETS_DIR, GENERATED_DIR, RECIPES_DIR, PAGES_DIR, CACHE_DIR];

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
    /// The shape of the frame this edit is made in (#1278) — `"16:9"`,
    /// `"9:16"`, `"4:5"`, any `across:down` within 4:1 of square.
    ///
    /// **A shape, not a size.** How many pixels a render has is chosen per
    /// render; what shape they are in is a fact about the edit, since a title
    /// laid out upright is a different title landscape. Every default follows
    /// it: a render's size, a still's, the frame `describe` measures against,
    /// and both editors' preview. Chosen when the project is made (a platform
    /// sets it) and changed with [`crate::frame::refit`], never by a bare
    /// write, which would leave footage that filled the old frame in bars.
    ///
    /// Absent reads as 16:9, what every surface assumed before the field was
    /// there; a document is always written with it.
    #[serde(default)]
    pub aspect: Ratio,
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
    /// Which document on disk this one is a change to — see [`Baseline`] for
    /// what [`Project::save`] does with it and why.
    ///
    /// **Never serialised**, and never part of the format: it is derived from
    /// the bytes the document was read from, so writing it into the document
    /// would be a file describing its own hash. A project read from another
    /// build, or hand-edited, needs no field for this to work.
    #[serde(skip)]
    pub baseline: Baseline,
}

impl Project {
    /// An empty project on the given grid, at the current schema version.
    pub fn new(name: impl Into<String>, timeline_fps: Fps) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            timeline_fps,
            aspect: Ratio::WIDE,
            script: None,
            assets: Vec::new(),
            tracks: Vec::new(),
            baseline: Baseline::unread(),
        }
    }

    /// Creates a `*.scor/` directory: the sub-directories and a
    /// `project.json` for an empty project on the given grid, in the given
    /// shape. Refuses a
    /// directory that already holds anything, a project or not, before a
    /// single file is written.
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
        aspect: Ratio,
    ) -> Result<Self, SaveError> {
        vacant(project_dir)?;
        for directory in PROJECT_DIRS {
            let path = project_dir.join(directory);
            fs::create_dir_all(&path).map_err(|source| SaveError::Io { path, source })?;
        }
        let mut project = Self::new(name_for(project_dir, name), timeline_fps);
        project.aspect = aspect;
        project.save(project_dir)?;
        Ok(project)
    }

    /// Looks an asset up by id.
    pub fn asset(&self, id: &AssetId) -> Option<&Asset> {
        self.assets.iter().find(|asset| &asset.id == id)
    }

    /// [`Project::asset`], to change.
    pub fn asset_mut(&mut self, id: &AssetId) -> Option<&mut Asset> {
        self.assets.iter_mut().find(|asset| &asset.id == id)
    }

    /// Whether the assets table has an entry called `id`.
    pub fn has_asset(&self, id: &AssetId) -> bool {
        self.asset(id).is_some()
    }

    /// Every note in the document, in reading order: the assets table first,
    /// then each track followed by the clips on it — the timeline's, then every
    /// group's, so the reasoning written on a member is read too.
    ///
    /// One pass over the whole document rather than three lists, because the
    /// question anyone asks is *what does this project say about itself?* and
    /// the answer is not sorted by which table a note happens to live in.
    pub fn notes(&self) -> impl Iterator<Item = Noted<'_>> {
        let assets = self.assets.iter().filter_map(Noted::on_asset);
        let tracks = self.every_track().flat_map(|track| {
            Noted::on_track(track)
                .into_iter()
                .chain(track.clips.iter().filter_map(Noted::on_clip))
        });
        assets.chain(tracks)
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
        Self::read(project_dir, Self::validate)
    }

    /// Reads `project.json` to **measure it again**: everything
    /// [`Project::load`] checks, except what reads the `media` blocks a probe
    /// is about to replace.
    ///
    /// A clip held to its source's recorded length is the case (#1007): a file
    /// regenerated outside scorsese grows, a clip grows with it, and the
    /// recorded length — now stale — makes the project fail to load. Re-probing
    /// is what fixes it, so the command that re-probes cannot be refused by it.
    /// The checks are held back, not dropped: whoever loads this way measures
    /// and then validates before saving, and a file that really is too short
    /// is refused then, against its real length.
    pub fn load_to_measure(project_dir: &Path) -> Result<Self, LoadError> {
        Self::read(project_dir, |project| project.unmeasured().validate())
    }

    /// The document as it would be if no file in it had ever been measured —
    /// what is left for validation to read is the document alone.
    fn unmeasured(&self) -> Self {
        let mut bare = self.clone();
        for asset in &mut bare.assets {
            asset.media = None;
        }
        bare
    }

    fn read(
        project_dir: &Path,
        check: impl FnOnce(&Self) -> Result<(), ValidationErrors>,
    ) -> Result<Self, LoadError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        let json = fs::read_to_string(&file).map_err(|source| LoadError::Io {
            path: file.clone(),
            source,
        })?;
        let project = Self::from_json(&json)?;
        check(&project)?;
        // Recorded here and nowhere else in this file: `from_json` parses a
        // string that was not necessarily read from anything, and inventing a
        // baseline for one would be asserting something nobody checked.
        project.baseline.moved_to(json.as_bytes());
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
    ///
    /// **Refuses to publish over a document this one is not a change to.** The
    /// file is re-read and compared against the [`Baseline`] before anything
    /// is written, so a save built on a read that something else has since
    /// overtaken fails loudly instead of taking the other writer's work with
    /// it. It is a compare-and-swap and not a transaction — [`Baseline`] says
    /// exactly how far the promise goes.
    pub fn save(&self, project_dir: &Path) -> Result<(), SaveError> {
        let file = project_dir.join(PROJECT_FILE_NAME);
        let json = self.to_json()?;
        self.baseline.guard(&file)?;
        crate::write::atomically(&file, &json)
            .map_err(|source| SaveError::Io { path: file, source })?;
        // What this document is a change to is now what it just wrote: an
        // editor saving twice is one writer, and the second save must be
        // measured against the first rather than against what preceded both.
        self.baseline.moved_to(json.as_bytes());
        Ok(())
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
    /// advice is how to get past it rather than a complaint about a field:
    /// migrate an older one, upgrade scorsese for a newer one.
    #[error(
        "project.json is schema_version {found}, but this build of scorsese reads {supported}{}",
        version_advice(*found, *supported)
    )]
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

/// What to do about a document at `found` when this build reads `supported`.
///
/// Only the local fix is named for an older one: the web migrates every
/// stored project when it starts, so nobody there ever reads this (#1115).
fn version_advice(found: u32, supported: u32) -> &'static str {
    if found < supported {
        " — `scorsese migrate --project <dir>` brings it forward"
    } else {
        " — it was written by a newer scorsese; upgrade this one to open it"
    }
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
    #[error("{} is already a scorsese project; nothing was changed", path.display())]
    AlreadyAProject {
        /// The directory that already holds a `project.json`.
        path: PathBuf,
    },
    /// Creating a project in a directory that already holds something else.
    /// Refused rather than laid over: half the directory would be someone
    /// else's, with nothing to say which half.
    #[error(
        "{} is not empty, so nothing was written. A project directory is laid out \
         from scratch — name one that does not exist yet.",
        path.display()
    )]
    NotEmpty {
        /// The directory that holds something already.
        path: PathBuf,
    },
    /// Somebody else saved this project since it was read, so publishing would
    /// take their work with it. Nothing was written.
    #[error(
        "{} changed since it was read, and saving would drop that change — nothing was \
         written. Read the project again and redo this edit on what is there now",
        path.display()
    )]
    Stale {
        /// The `project.json` that has moved on.
        path: PathBuf,
    },
    /// A document that was never read from this file, saved over one that is
    /// there. Nothing was written.
    ///
    /// Its own variant rather than [`SaveError::Stale`]: nothing *changed*
    /// here, and telling a caller to re-read something it never read would
    /// send it looking for a difference it cannot find.
    #[error(
        "refusing to overwrite {}: this document was not read from it, so there is no \
         telling what is already there — nothing was written. Read the project and save \
         with the fingerprint that read reported",
        path.display()
    )]
    Unread {
        /// The `project.json` that would have been overwritten blind.
        path: PathBuf,
    },
}
