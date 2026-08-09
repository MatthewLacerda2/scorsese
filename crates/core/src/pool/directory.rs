//! Importing what a directory holds, one asset per file.
//!
//! A directory is a convenience for the *call*. It never becomes an asset:
//! assets are the things a clip references and the compositor draws or hears,
//! and a folder has no duration, no pixels and no samples — so a folder in the
//! assets table would be a kind nothing can use and every consumer of the
//! table would grow a branch for the case that never renders.
//!
//! Three rules make a folder import predictable enough to hand an agent:
//!
//! - **It does not recurse.** "What is in this folder" is a flat, obvious
//!   thing; walking a tree invents structure nobody asked for and can pull in
//!   far more than was meant.
//! - **It is sorted by file name**, so the same folder imports to the same ids
//!   in the same order on every machine.
//! - **It refuses as a whole or not at all.** Everything that can reject a
//!   file — an id already taken, an unreadable file, an extension that lies —
//!   is found before the first byte is copied, so a refusal leaves the project
//!   directory exactly as it was rather than half a folder in `assets/`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::{AssetId, AssetKind, MediaMetadata};
use crate::probe::ProbeMedia;
use crate::project::Project;

use super::import::{
    ImportError, already_in_pool, hash_of, import_asset, measure, place, resolve_kind,
};
use super::naming::{infer_kind, wanted_asset_id};

/// Imports a file, or everything directly inside a directory.
///
/// This is the whole of what `scorsese import` and the `import` MCP tool do,
/// so both answer the same way about the same folder. `kind` overrides what
/// the extension says; for a directory it says what the *media* in it is, and
/// inference still decides which files count as media at all — otherwise
/// naming a kind would drag in the licence text and the `.DS_Store` beside the
/// footage.
pub fn import_path(
    project: &mut Project,
    project_root: &Path,
    path: &Path,
    kind: Option<AssetKind>,
    probe: &dyn ProbeMedia,
) -> Result<Import, ImportError> {
    if path.is_dir() {
        return import_directory(project, project_root, path, kind, probe);
    }
    let before = project.assets.len();
    let asked_for = wanted_asset_id(path);
    let id = import_asset(project, project_root, path, kind, probe)?;
    let reused = project.assets.len() == before;
    Ok(Import {
        imported: vec![Imported {
            // Reuse is not a rename. The id that comes back for content the
            // pool already holds is whatever *that* asset was called, which has
            // nothing to do with the name of the file just handed over — and
            // reporting it as a rename would put a warning on the one case that
            // is working exactly as intended.
            wanted: (!reused && id != asked_for).then_some(asked_for),
            id,
            source: file_name(path),
            reused,
        }],
        skipped: Vec::new(),
    })
}

/// What one call brought in, and what it passed over.
#[derive(Debug, Default)]
pub struct Import {
    /// The assets the call ended at, in the order the files were read.
    pub imported: Vec<Imported>,
    /// The files that were not media, named so that a mistyped video and a
    /// licence file do not look identical from the outside.
    pub skipped: Vec<Skipped>,
}

/// One file that is now an asset.
#[derive(Debug)]
pub struct Imported {
    /// The asset to reference from a clip.
    pub id: AssetId,
    /// The file's own name, outside the project.
    pub source: String,
    /// True when the pool already held these bytes: `id` is the asset that
    /// already existed and nothing was copied.
    pub reused: bool,
    /// The id the file's name asked for, and **only** when it did not get it.
    ///
    /// `Some` means something already answered to that id and this asset was
    /// suffixed out of the way — `intro.mp4` landing as `intro-2`. Suffixing is
    /// the right answer for a single file, and refusing over a name collision
    /// would make the common case fail for no good reason; what was missing was
    /// saying so. Whoever imported `intro.mp4` otherwise finds out when they go
    /// looking for `intro` and get the wrong clip.
    ///
    /// Always `None` for a directory import, where a taken id is refused
    /// outright before anything is copied. That asymmetry is deliberate and its
    /// reasoning is on [`ImportError::IdTaken`].
    pub wanted: Option<AssetId>,
}

/// One file that was passed over, and why.
#[derive(Debug)]
pub struct Skipped {
    /// The file's own name, outside the project.
    pub source: String,
    /// Why it was not brought in.
    pub why: SkipReason,
}

/// Why a file in a directory was not imported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A subdirectory, or something the filesystem does not offer as a file.
    /// Import does not recurse, so it is left where it is.
    NotAFile,
    /// No extension, or one nothing recognises. Guessing would import the file
    /// as the wrong kind and only surface at render time.
    UnknownKind,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotAFile => "not a file — import does not recurse into directories",
            Self::UnknownKind => "not media, as far as its extension says",
        })
    }
}

/// One file, decided about but not yet acted on.
enum Step {
    /// The pool already holds these bytes: nothing to copy.
    Reuse { id: AssetId, name: String },
    /// Checked and measured, waiting to be copied in.
    Copy(Planned),
}

/// A file that has been checked and measured and is waiting to be copied.
struct Planned {
    source: PathBuf,
    name: String,
    kind: AssetKind,
    sha256: String,
    media: MediaMetadata,
    id: AssetId,
}

fn import_directory(
    project: &mut Project,
    project_root: &Path,
    dir: &Path,
    kind: Option<AssetKind>,
    probe: &dyn ProbeMedia,
) -> Result<Import, ImportError> {
    let mut report = Import::default();
    let mut plan: Vec<Step> = Vec::new();

    for source in listing(dir)? {
        let name = file_name(&source);
        let Some(inferred) = importable(&source) else {
            let why = if source.is_file() {
                SkipReason::UnknownKind
            } else {
                SkipReason::NotAFile
            };
            report.skipped.push(Skipped { source: name, why });
            continue;
        };
        let kind = resolve_kind(&source, Some(kind.unwrap_or(inferred)))?;
        let sha256 = hash_of(&source)?;
        // Content already in the pool is that same asset, not a collision: an
        // import loop re-run is the thing an agent does by accident.
        if let Some(id) = already_in_pool(project, &sha256) {
            plan.push(Step::Reuse { id, name });
            continue;
        }
        let id = wanted_asset_id(&source);
        if project.asset(&id).is_some() || plan.iter().any(|step| step.claims(&id)) {
            return Err(ImportError::IdTaken { path: source, id });
        }
        let media = measure(&source, kind, probe)?;
        plan.push(Step::Copy(Planned {
            source,
            name,
            kind,
            sha256,
            media,
            id,
        }));
    }

    // Nothing above touched the project directory, so every refusal so far
    // left it exactly as it was. From here on it is copies and table rows.
    for step in plan {
        report.imported.push(match step {
            Step::Reuse { id, name } => Imported {
                id,
                source: name,
                reused: true,
                wanted: None,
            },
            // Never a rename: the id was checked for collisions above and the
            // whole batch refused if it was taken, so `place` is handed the id
            // that was planned and returns that same one.
            Step::Copy(planned) => Imported {
                id: place(
                    project,
                    project_root,
                    &planned.source,
                    planned.kind,
                    planned.sha256,
                    planned.media,
                    Some(planned.id),
                )?,
                source: planned.name,
                reused: false,
                wanted: None,
            },
        });
    }
    Ok(report)
}

impl Step {
    /// Whether an earlier file in this same directory already wants that id.
    fn claims(&self, id: &AssetId) -> bool {
        matches!(self, Self::Copy(planned) if &planned.id == id)
    }
}

/// The kind a file directly inside the directory would come in as, or `None`
/// when it is not media the project can use.
fn importable(source: &Path) -> Option<AssetKind> {
    source.is_file().then(|| infer_kind(source)).flatten()
}

/// Everything directly inside the directory, sorted by file name — so the same
/// folder imports to the same ids in the same order every time, whatever order
/// the filesystem happens to hand entries back in.
fn listing(dir: &Path) -> Result<Vec<PathBuf>, ImportError> {
    let unreadable = |source: std::io::Error| ImportError::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(unreadable)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(unreadable)?;
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(entries)
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
