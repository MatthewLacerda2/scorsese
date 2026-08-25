//! Bringing outside media into the project.

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::{Asset, AssetId, AssetKind, MediaMetadata};
use crate::path::ProjectPath;
use crate::probe::{ProbeError, ProbeMedia};
use crate::project::{ASSETS_DIR, Project};

use super::hash::hash_file;
use super::naming::{infer_kind, unique_asset_id, unique_file_name};
use super::probing::as_recorded;

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
    let sha256 = hash_of(source)?;
    if let Some(existing) = already_in_pool(project, &sha256) {
        return Ok(existing);
    }
    // Measure before copying. Everything that can reject a file happens while
    // the project directory is still untouched, so a failed import cannot
    // leave a stray file behind in `assets/`.
    let media = measure(source, kind, probe)?;
    place(project, project_root, source, kind, sha256, media, None)
}

/// The file's content hash, worded as an import failure.
pub(super) fn hash_of(source: &Path) -> Result<String, ImportError> {
    hash_file(source).map_err(|source_err| ImportError::Unreadable {
        path: source.to_path_buf(),
        source: source_err,
    })
}

/// The asset already holding this content, if the pool has it.
pub(super) fn already_in_pool(project: &Project, sha256: &str) -> Option<AssetId> {
    project
        .assets
        .iter()
        .find(|a| a.sha256.as_deref() == Some(sha256))
        .map(|a| a.id.clone())
}

/// Reads the file and checks it really is what it is being imported as.
///
/// Separate from [`place`] so a caller bringing in a whole directory can find
/// out that one file in it is unreadable while nothing has been copied yet.
pub(super) fn measure(
    source: &Path,
    kind: AssetKind,
    probe: &dyn ProbeMedia,
) -> Result<MediaMetadata, ImportError> {
    let media = probe.probe(source)?;
    check_kind_against_media(kind, &media, source)?;
    // What a still's metadata says is decided in one place, shared with
    // `scorsese probe` — see [`as_recorded`]. Two paths into the assets table
    // that describe one file differently would be a bug nobody could see.
    Ok(as_recorded(kind, media))
}

/// Copies the measured file into `assets/` and records it in the table.
///
/// `id` is the id to record it under. `None` derives one from the name the
/// file lands under, suffixing until it is free — which is what a single-file
/// import does. A caller that planned its ids ahead of copying anything passes
/// the one it planned, so the id it checked for collisions is the id it gets.
pub(super) fn place(
    project: &mut Project,
    project_root: &Path,
    source: &Path,
    kind: AssetKind,
    sha256: String,
    media: MediaMetadata,
    id: Option<AssetId>,
) -> Result<AssetId, ImportError> {
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

    let id = id.unwrap_or_else(|| unique_asset_id(project, &file_name));
    let path = ProjectPath::new(format!("{ASSETS_DIR}/{file_name}"));
    project.assets.push(Asset {
        sha256: Some(sha256),
        media: Some(media),
        ..Asset::imported(id.clone(), kind, path)
    });
    Ok(id)
}

pub(super) fn resolve_kind(
    source: &Path,
    requested: Option<AssetKind>,
) -> Result<AssetKind, ImportError> {
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
    media: &MediaMetadata,
    source: &Path,
) -> Result<(), ImportError> {
    let expectation = if kind.is_visual() && media.width.is_none() {
        Some("no video stream")
    } else if kind.is_visual() && has_no_measurable_picture(media) {
        Some("no picture ffmpeg can read")
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

/// A picture the prober recognised but could not measure.
///
/// ffprobe answers `0x0` for a file whose container it reads and whose frames
/// it cannot decode. An **animated webp** is what brought this in: ffmpeg has
/// no decoder for the animation, so it reports neither the size nor a single
/// pixel, while still looking enough like a picture to pass the check above.
///
/// It is worth its own refusal because of how it fails otherwise. A file with
/// no decodable frame does not stop a render — the decoder waits on frames
/// that never arrive, and an unattended render hangs instead of failing. One
/// refused import at the door is the cheap end of that.
fn has_no_measurable_picture(media: &MediaMetadata) -> bool {
    media.width == Some(0) || media.height == Some(0)
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
    /// A file's id is one an asset in the pool already answers to.
    ///
    /// Raised only when a whole directory is imported, and raised before
    /// anything is copied, so the refusal changes nothing at all. A single
    /// file names one asset and can be suffixed out of the way; a directory
    /// is a batch, and quietly suffixing one file in it adds a second asset
    /// nobody asked for and nobody would notice.
    #[error("{} would import as `{id}`, which is already an asset", path.display())]
    IdTaken {
        /// The file that could not be brought in.
        path: PathBuf,
        /// The id it would have taken.
        id: AssetId,
    },
    /// The prober could not read the file at all.
    #[error(transparent)]
    Probe(#[from] ProbeError),
}
