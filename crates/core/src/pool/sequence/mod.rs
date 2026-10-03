//! Bringing in a folder of frames as **one** image sequence, and making or
//! changing a sequence from stills already in the pool.
//!
//! A folder imported the ordinary way ([`super::import_path`]) is a batch of
//! separate stills, sorted by name, and nothing more. Imported as a sequence it
//! is the same stills — each an `image` asset, hashed and probed exactly as an
//! ordinary import does it — plus the `image_sequence` asset that plays them,
//! in the order their **numbers** say. The stills are copied into
//! `assets/<sequence id>/` rather than loose into `assets/`, so a timelapse's
//! four hundred photographs stay one folder on disk too, and their ids carry
//! the sequence's as a prefix.
//!
//! It keeps the directory import's three promises: it does not recurse, the
//! same folder imports the same way on every machine, and it **refuses as a
//! whole or not at all** — everything that can turn a frame away is found
//! before the first byte is copied.

mod edit;
mod order;

use std::fs;
use std::path::{Path, PathBuf};

use crate::asset::sequence::same_format;
use crate::asset::{Asset, AssetId, AssetKind, ImageSequence, MediaMetadata, SEQUENCE_FORMATS};
use crate::path::ProjectPath;
use crate::probe::ProbeMedia;
use crate::project::{ASSETS_DIR, Project};

use super::directory::{SkipReason, Skipped};
use super::import::{ImportError, already_in_pool, hash_of, measure};
use super::naming::{asset_id_for, sanitise, unique_file_name};

pub use edit::{SequenceChange, SequenceChanged, SequenceError, change_sequence};
pub use order::Gap;

/// What importing a folder as a sequence did.
#[derive(Debug)]
pub struct SequenceImport {
    /// The `image_sequence` asset — the one a clip shows.
    pub id: AssetId,
    /// Every still, in the order it plays.
    pub stills: Vec<AssetId>,
    /// How many of them the pool already held, so nothing was copied.
    pub reused: usize,
    /// True when the pool already held a sequence of exactly these stills and
    /// that is what came back: nothing at all was added.
    pub existed: bool,
    /// Where the numbering skips. Reported, never refused: a frame missing from
    /// a numbered run is nearly always one somebody deleted on purpose.
    pub gaps: Vec<Gap>,
    /// What was in the folder and is not a frame.
    pub skipped: Vec<Skipped>,
}

/// One frame, checked and measured.
struct Frame {
    source: PathBuf,
    name: String,
    sha256: String,
    media: MediaMetadata,
}

/// Imports the frames directly inside `dir` as stills, and adds the sequence
/// that plays them: each held one frame, played once — the shape a rendered
/// frame directory and a timelapse both arrive in. [`change_sequence`] sets
/// any other hold or a loop.
///
/// Refused, with nothing copied, when the folder holds no frames, or frames of
/// two formats or two sizes — the same three things validation refuses in a
/// sequence, caught while it is still a folder rather than a broken asset.
pub fn import_sequence(
    project: &mut Project,
    project_root: &Path,
    dir: &Path,
    probe: &dyn ProbeMedia,
) -> Result<SequenceImport, ImportError> {
    let (frames, skipped) = frames_in(dir, probe)?;
    let names: Vec<String> = frames.iter().map(|frame| frame.name.clone()).collect();
    let gaps = order::gaps(&names);

    let known: Vec<Option<AssetId>> = frames
        .iter()
        .map(|frame| already_in_pool(project, &frame.sha256))
        .collect();
    if let Some(stills) = known.iter().cloned().collect::<Option<Vec<_>>>()
        && let Some(existing) = project
            .assets
            .iter()
            .find(|asset| asset.sequence.as_ref().is_some_and(|s| s.stills == stills))
    {
        return Ok(SequenceImport {
            id: existing.id.clone(),
            reused: stills.len(),
            stills,
            existed: true,
            gaps,
            skipped,
        });
    }

    let id = asset_id_for(project, &folder_name(dir));
    let folder = project_root.join(ASSETS_DIR).join(id.as_str());
    let mut stills = Vec::with_capacity(frames.len());
    let mut reused = 0;
    for (frame, known) in frames.into_iter().zip(known) {
        // Checked again rather than trusted: a folder holding the same
        // picture twice — a held drawing saved as two files — is one asset.
        if let Some(existing) = known.or_else(|| already_in_pool(project, &frame.sha256)) {
            reused += 1;
            stills.push(existing);
            continue;
        }
        stills.push(place(project, &folder, &id, frame)?);
    }
    project.assets.push(Asset::image_sequence(
        id.clone(),
        ImageSequence::new(stills.clone()),
    ));
    Ok(SequenceImport {
        id,
        stills,
        reused,
        existed: false,
        gaps,
        skipped,
    })
}

/// Every frame in the folder, in the order it plays, checked and measured —
/// and every file passed over. Nothing is copied here.
fn frames_in(
    dir: &Path,
    probe: &dyn ProbeMedia,
) -> Result<(Vec<Frame>, Vec<Skipped>), ImportError> {
    let refuse = |why: String| ImportError::NotASequence {
        path: dir.to_path_buf(),
        why,
    };
    let unreadable = |source| ImportError::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(unreadable)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(unreadable)?;
    entries.sort_by(|a, b| order::natural(&name_of(a), &name_of(b)));

    let (mut frames, mut skipped) = (Vec::new(), Vec::new());
    let mut first: Option<(String, Option<(u32, u32)>)> = None;
    for source in entries {
        let name = name_of(&source);
        let format = format_of(&source);
        if !source.is_file() || !SEQUENCE_FORMATS.contains(&format.as_str()) {
            let why = if !source.is_file() {
                SkipReason::NotAFile
            } else {
                SkipReason::NotAFrame
            };
            skipped.push(Skipped { source: name, why });
            continue;
        }
        let sha256 = hash_of(&source)?;
        let media = measure(&source, AssetKind::Image, probe)?;
        let size = media.width.zip(media.height);
        match &first {
            None => first = Some((format.clone(), size)),
            Some((expected, _)) if !same_format(expected, &format) => {
                return Err(refuse(format!(
                    "{name} is .{format} among .{expected} frames — a sequence is one format"
                )));
            }
            Some((_, Some((w, h)))) if size.is_some_and(|size| size != (*w, *h)) => {
                return Err(refuse(format!(
                    "{name} is not {w}x{h} like the frames before it — a sequence is one size"
                )));
            }
            Some(_) => {}
        }
        frames.push(Frame {
            source,
            name,
            sha256,
            media,
        });
    }
    if frames.is_empty() {
        return Err(refuse(format!(
            "it holds no frames: {}",
            SEQUENCE_FORMATS.join(", ")
        )));
    }
    Ok((frames, skipped))
}

/// Copies one frame into the sequence's folder and records it as a still.
fn place(
    project: &mut Project,
    folder: &Path,
    sequence: &AssetId,
    frame: Frame,
) -> Result<AssetId, ImportError> {
    let unwritable = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ImportError::Unwritable { path, source }
    };
    fs::create_dir_all(folder).map_err(unwritable(folder))?;
    let file_name = unique_file_name(folder, &frame.source);
    let destination = folder.join(&file_name);
    fs::copy(&frame.source, &destination).map_err(unwritable(&destination))?;

    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name.as_str(), |(stem, _)| stem);
    let id = asset_id_for(project, &format!("{sequence}-{stem}"));
    let path = ProjectPath::new(format!("{ASSETS_DIR}/{sequence}/{file_name}"));
    project.assets.push(Asset {
        sha256: Some(frame.sha256),
        media: Some(frame.media),
        ..Asset::imported(id.clone(), AssetKind::Image, path)
    });
    Ok(id)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// What the folder is called, which is what the sequence is called.
fn folder_name(dir: &Path) -> String {
    let name = std::path::absolute(dir)
        .ok()
        .and_then(|dir| {
            dir.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    if name.is_empty() {
        "sequence".to_owned()
    } else {
        sanitise(&name)
    }
}

fn format_of(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}
