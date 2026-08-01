//! Choosing asset ids and file names that stay readable and stay unique.

use std::path::Path;

use crate::asset::{AssetId, AssetKind};
use crate::project::Project;

const VIDEO: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpg", "mpeg", "wmv",
];
const IMAGE: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "avif",
];
const AUDIO: &[&str] = &[
    "mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "aiff", "wma",
];

/// Guesses a kind from the file extension. The probe checks the guess against
/// what the file actually contains, so a wrong extension is caught rather
/// than trusted.
pub(super) fn infer_kind(path: &Path) -> Option<AssetKind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let extension = extension.as_str();
    if VIDEO.contains(&extension) {
        Some(AssetKind::Video)
    } else if IMAGE.contains(&extension) {
        Some(AssetKind::Image)
    } else if AUDIO.contains(&extension) {
        Some(AssetKind::Audio)
    } else {
        None
    }
}

/// A file name for `assets/` that does not collide with what is already
/// there. `logo.png` becomes `logo-2.png` when taken.
pub(super) fn unique_file_name(assets_dir: &Path, source: &Path) -> String {
    let name = sanitise(
        source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset"),
    );
    let (stem, extension) = split_extension(&name);
    let mut candidate = name.clone();
    let mut suffix = 2;
    while assets_dir.join(&candidate).exists() {
        candidate = join_extension(&format!("{stem}-{suffix}"), extension);
        suffix += 1;
    }
    candidate
}

/// A legible, unique id for a new asset, from a name somebody chose.
///
/// The public door onto the same rules import follows — sanitised so the id
/// stays portable, then suffixed until it is unused. For the kinds that are
/// *named* into existence rather than imported from a file.
pub fn asset_id_for(project: &Project, desired: &str) -> AssetId {
    unique_asset_id(project, &sanitise(desired))
}

/// The id a source file asks for, before anything checks whether it is free.
///
/// Separate from [`unique_asset_id`] because a caller importing a whole
/// directory has to know what each file *wants* to be called while it can
/// still refuse — a suffix applied on the way in is a decision made after the
/// point where refusing was possible.
pub(super) fn wanted_asset_id(source: &Path) -> AssetId {
    let name = sanitise(
        source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset"),
    );
    let (stem, _) = split_extension(&name);
    AssetId::new(if stem.is_empty() { "asset" } else { stem }.to_owned())
}

/// An asset id derived from the file name, unique within the project. Ids are
/// what a human types into `project.json`, so they stay legible rather than
/// becoming hashes.
pub(super) fn unique_asset_id(project: &Project, file_name: &str) -> AssetId {
    let (stem, _) = split_extension(file_name);
    let base = if stem.is_empty() { "asset" } else { stem };
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while project.assets.iter().any(|a| a.id.as_str() == candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    AssetId::new(candidate)
}

/// Keeps names portable: lowercase, and anything that is not alphanumeric,
/// `-`, `_`, or `.` becomes a hyphen. Paths inside a project must not depend
/// on a filesystem's tolerance for spaces or colons.
fn sanitise(name: &str) -> String {
    let cleaned: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "asset".to_owned()
    } else {
        trimmed
    }
}

fn split_extension(name: &str) -> (&str, Option<&str>) {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, Some(extension)),
        _ => (name, None),
    }
}

fn join_extension(stem: &str, extension: Option<&str>) -> String {
    match extension {
        Some(extension) => format!("{stem}.{extension}"),
        None => stem.to_owned(),
    }
}
