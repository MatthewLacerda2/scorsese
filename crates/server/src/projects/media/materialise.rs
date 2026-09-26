//! Rendering a stored project: lay it out as a `.scor` folder for a moment.
//!
//! The render pipeline reads a project directory — `project.json` beside the
//! files it names — and that is the only shape it needs to know. So a stored
//! project is rendered by **building that directory**, temporarily: its
//! document written out, and each file it uses **linked** (never copied) from
//! where the user's storage keeps it. `scorsese-render` and the compositor
//! run on it unchanged, which is the point: the web app renders with the same
//! code as the CLI, and neither learns about Postgres. Teaching the renderer
//! to find media by hash directly is a possible later optimisation, not a
//! requirement.
//!
//! Links are **symbolic**, to absolute paths: the folder never leaves the
//! machine and never outlives the render, so the "no absolute paths" rule —
//! which is about the *document* surviving a move — is not in play, and the
//! document inside still names everything relatively. A symlink works across
//! filesystems, where scratch space and the library may well be on different
//! disks; a hard link would not.
//!
//! What starts a render, and where the folder goes, is the job queue's
//! (#536, #541). This only builds and removes the folder.

use std::io;
use std::path::{Path, PathBuf};

use scorsese_core::{
    ASSETS_DIR, AssetId, CACHE_DIR, GENERATED_DIR, PROJECT_FILE_NAME, Project, ProjectPath,
    RECIPES_DIR,
};

/// Where the bytes of a user's file with a given hash are, if they have one.
///
/// The caller's answer, scoped to the one user whose project this is — so a
/// document naming a hash it does not own finds nothing.
pub trait MediaSource {
    /// The file with this `sha256`, if the user has one.
    fn locate(&self, sha256: &str) -> Option<PathBuf>;
}

impl<F: Fn(&str) -> Option<PathBuf>> MediaSource for F {
    fn locate(&self, sha256: &str) -> Option<PathBuf> {
        self(sha256)
    }
}

/// A project laid out as a `.scor` folder, removed when this is dropped.
///
/// Removing it removes the links and never what they point at: a directory
/// removal does not follow symbolic links.
#[derive(Debug)]
pub struct Materialised {
    root: PathBuf,
    missing: Vec<AssetId>,
}

impl Materialised {
    /// The folder: what to hand the renderer.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Assets that name a hash the user has no file for. Left absent rather
    /// than refused, so the renderer's own check names them the way it names
    /// a missing file in any project.
    pub fn missing(&self) -> &[AssetId] {
        &self.missing
    }
}

impl Drop for Materialised {
    fn drop(&mut self) {
        // Nothing to report to: a leftover folder is scratch, and the next
        // clean-up of the scratch directory takes it.
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Why a project could not be laid out.
#[derive(Debug, thiserror::Error)]
pub enum MaterialiseError {
    /// Something is already at the folder's path. Never written into, since
    /// dropping the result would delete whatever it was.
    #[error("{} already exists", .0.display())]
    Exists(PathBuf),

    /// An asset's path is absolute or climbs out of the project. Refused
    /// rather than linked: it would put a link outside the folder.
    #[error("asset {asset} has a path that is not project-relative: {path}")]
    BadPath {
        /// The asset.
        asset: AssetId,
        /// Its path.
        path: ProjectPath,
    },

    /// The document could not be serialised.
    #[error("serialising the project: {0}")]
    Serialize(#[from] serde_json::Error),

    /// The filesystem refused.
    #[error("{}: {source}", path.display())]
    Io {
        /// What was being made.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error,
    },
}

/// Lay `project` out at `at`, a path nothing is at yet, linking every file it
/// names by hash to where `media` says it is.
pub fn materialise(
    project: &Project,
    at: &Path,
    media: &impl MediaSource,
) -> Result<Materialised, MaterialiseError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| MaterialiseError::Io { path, source }
    };
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    match std::fs::create_dir(at) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(MaterialiseError::Exists(at.to_path_buf()));
        }
        other => other.map_err(io(at))?,
    }
    // From here on the folder is ours, and an early return removes it.
    let mut folder = Materialised {
        root: at.to_path_buf(),
        missing: Vec::new(),
    };
    for directory in [ASSETS_DIR, GENERATED_DIR, RECIPES_DIR, CACHE_DIR] {
        let path = at.join(directory);
        std::fs::create_dir(&path).map_err(io(&path))?;
    }
    // The document first, so no asset path — `project.json` included — can
    // put a link where it goes and have the write land through it.
    let document = at.join(PROJECT_FILE_NAME);
    std::fs::write(&document, project.to_json()?).map_err(io(&document))?;

    for asset in &project.assets {
        let (Some(path), Some(hash)) = (&asset.path, &asset.sha256) else {
            continue;
        };
        if path.check().is_err() {
            return Err(MaterialiseError::BadPath {
                asset: asset.id.clone(),
                path: path.clone(),
            });
        }
        let Some(source) = media.locate(hash) else {
            folder.missing.push(asset.id.clone());
            continue;
        };
        let target = path.resolve(at);
        // Two assets on one path are one file: the first link stands.
        if target.symlink_metadata().is_ok() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        let source = std::path::absolute(&source).map_err(io(&source))?;
        link(&source, &target).map_err(io(&target))?;
    }
    Ok(folder)
}

#[cfg(unix)]
fn link(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

/// A symbolic link needs privileges on Windows; a hard link does not, and the
/// server does not run there anyway.
#[cfg(not(unix))]
fn link(source: &Path, target: &Path) -> io::Result<()> {
    std::fs::hard_link(source, target)
}
