//! Bringing outside media into the project.

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::{Asset, AssetId, AssetKind};
use crate::path::ProjectPath;
use crate::probe::{ProbeError, ProbeMedia};
use crate::project::{ASSETS_DIR, Project};

use super::hash::hash_file;
use super::naming::{infer_kind, unique_asset_id, unique_file_name};

/// Copies a file into the project's `assets/`, hashes and probes it, and adds
/// it to the assets table. Returns the id of the asset to reference.
///
/// Import is a **copy**, never a link: the project directory stays
/// self-contained so it survives `scp -r`. Importing a file whose content is
/// already in the pool returns the existing asset instead of copying again —
/// assets are entities, and two clips pointing at one entity is the intended
/// shape, not duplication.
pub fn import_asset(
    project: &mut Project,
    project_root: &Path,
    source: &Path,
    kind: Option<AssetKind>,
    probe: &dyn ProbeMedia,
) -> Result<AssetId, ImportError> {
    let kind = resolve_kind(source, kind)?;
    let sha256 = hash_file(source).map_err(|source_err| ImportError::Unreadable {
        path: source.to_path_buf(),
        source: source_err,
    })?;

    if let Some(existing) = project
        .assets
        .iter()
        .find(|a| a.sha256.as_ref() == Some(&sha256))
    {
        return Ok(existing.id.clone());
    }

    // Probe before copying. Everything that can reject a file happens while
    // the project directory is still untouched, so a failed import cannot
    // leave a stray file behind in `assets/`.
    let mut media = probe.probe(source)?;
    check_kind_against_media(kind, &media, source)?;
    if kind == AssetKind::Image {
        // ffprobe reports a still as a one-frame video, inventing a frame
        // rate and sometimes a duration. A still has neither; how long it is
        // on screen is the clip's business, not the file's.
        media.frame_rate = None;
        media.duration_seconds = None;
    }

    let assets_dir = project_root.join(ASSETS_DIR);
    fs::create_dir_all(&assets_dir).map_err(|source_err| ImportError::Unwritable {
        path: assets_dir.clone(),
        source: source_err,
    })?;

    let file_name = unique_file_name(&assets_dir, source);
    let destination = assets_dir.join(&file_name);
    fs::copy(source, &destination).map_err(|source_err| ImportError::Unwritable {
        path: destination.clone(),
        source: source_err,
    })?;

    let id = unique_asset_id(project, &file_name);
    let path = ProjectPath::new(format!("{ASSETS_DIR}/{file_name}"));
    project.assets.push(Asset {
        sha256: Some(sha256),
        media: Some(media),
        ..Asset::imported(id.clone(), kind, path)
    });
    Ok(id)
}

fn resolve_kind(source: &Path, requested: Option<AssetKind>) -> Result<AssetKind, ImportError> {
    let kind = match requested {
        Some(kind) => kind,
        None => infer_kind(source).ok_or_else(|| ImportError::UnknownKind {
            path: source.to_path_buf(),
        })?,
    };
    // A prompt is authored and an inline kind is written down; neither has a
    // file to copy in.
    if kind.is_generated() || !kind.is_file_backed() {
        return Err(ImportError::NotImportable { kind });
    }
    Ok(kind)
}

/// Catches a file whose extension lies — a `.mp3` that is really a video, or
/// a `.mp4` with no picture in it.
fn check_kind_against_media(
    kind: AssetKind,
    media: &crate::asset::MediaMetadata,
    source: &Path,
) -> Result<(), ImportError> {
    let expectation = if kind.is_visual() && media.width.is_none() {
        Some("no video stream")
    } else if kind.is_audible() && media.audio_channels.is_none() {
        Some("no audio stream")
    } else {
        None
    };
    match expectation {
        Some(found) => Err(ImportError::KindMismatch {
            path: source.to_path_buf(),
            kind,
            found,
        }),
        None => Ok(()),
    }
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The source file could not be read to hash it. Raised before anything
    /// is copied, so the project directory is untouched.
    #[error("cannot read {}: {source}", path.display())]
    Unreadable {
        /// The source file outside the project.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// `assets/` could not be created, or the copy into it failed.
    #[error("cannot write {}: {source}", path.display())]
    Unwritable {
        /// The destination that could not be written.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// No extension, or one nothing recognises. Guessing here would import a
    /// file as the wrong kind and only surface at render time.
    #[error("cannot tell what kind of media {} is; pass the kind explicitly", path.display())]
    UnknownKind {
        /// The file whose extension said nothing useful.
        path: PathBuf,
    },
    /// A `text` or `generated_*` kind was asked for. Those carry a string, not
    /// a file, so there is nothing for import to do with them.
    #[error("{kind:?} assets are authored, not imported — there is no file to copy in")]
    NotImportable {
        /// The kind that has no file behind it.
        kind: AssetKind,
    },
    /// The extension lied — a `.mp4` with no picture, a `.mp3` with no sound.
    /// Caught at import, when it is still one file rather than a broken cut.
    #[error("{} was imported as {kind:?} but has {found}", path.display())]
    KindMismatch {
        /// The file that was probed.
        path: PathBuf,
        /// The kind it was being imported as.
        kind: AssetKind,
        /// What the probe found instead, e.g. `no audio stream`.
        found: &'static str,
    },
    /// The prober could not read the file at all.
    #[error(transparent)]
    Probe(#[from] ProbeError),
}
