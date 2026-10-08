//! Bringing files down: a preview to look at, a small rendition to look
//! through, and the chosen file itself, imported.

use std::fs::File;
use std::path::{Path, PathBuf};

use scorsese_core::{Imported, ProbeMedia, Project, import_path};

use super::candidate::{Candidate, Medium, Rendition};
use super::library::{Library, StockError};
use super::search::find;

/// The most a preview JPEG may be. They are tens of kilobytes.
const PREVIEW_LIMIT: u64 = 8 * 1024 * 1024;

/// The most a rendition looked through may be: the smallest one, which is a
/// few megabytes for a minute of footage.
const LOOK_LIMIT: u64 = 256 * 1024 * 1024;

/// The most a chosen video may be. A minute of 4K is a few hundred megabytes.
const VIDEO_LIMIT: u64 = 4 * 1024 * 1024 * 1024;

/// The most a chosen picture may be.
const IMAGE_LIMIT: u64 = 256 * 1024 * 1024;

/// Downloads `url` to `path`, whole or not at all.
fn download(
    library: &dyn Library,
    url: &str,
    limit: u64,
    path: &Path,
) -> Result<PathBuf, StockError> {
    let io = |source| StockError::Io {
        path: path.to_path_buf(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    // Into a sibling first, so a download that dies halfway never leaves a
    // file that looks whole under the name a later call reuses.
    let partial = path.with_extension("part");
    let written = File::create(&partial)
        .map_err(io)
        .and_then(|mut file| library.download(url, limit, &mut file));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&partial);
        return Err(error);
    }
    std::fs::rename(&partial, path).map_err(io)?;
    Ok(path.to_path_buf())
}

/// Each candidate's preview JPEG, under the cache — `None` for one that has
/// none or would not download, which a sheet then leaves out rather than
/// failing every other picture over.
pub fn previews(
    cache: &Path,
    library: &dyn Library,
    candidates: &[Candidate],
) -> Vec<Option<PathBuf>> {
    candidates
        .iter()
        .map(|one| {
            if one.preview_url.is_empty() {
                return None;
            }
            let path = cache
                .join("previews")
                .join(format!("{}-{}.jpg", one.medium.word(), one.id));
            if path.is_file() {
                return Some(path);
            }
            download(library, &one.preview_url, PREVIEW_LIMIT, &path).ok()
        })
        .collect()
}

/// The smallest rendition of video `id`, under the cache, for a contact
/// sheet to be taken of before anything is imported.
pub fn footage(
    cache: &Path,
    library: &dyn Library,
    id: u64,
) -> Result<(Candidate, PathBuf), StockError> {
    let candidate = find(cache, library, Medium::Video, id)?;
    let smallest = candidate.renditions.first().ok_or(StockError::NoFile {
        library: library.name(),
        medium: Medium::Video,
        id,
    })?;
    let path = cache.join("looks").join(format!("video-{id}.mp4"));
    let path = if path.is_file() {
        path
    } else {
        download(library, &smallest.url.clone(), LOOK_LIMIT, &path)?
    };
    Ok((candidate, path))
}

/// One result to import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// Footage or a picture.
    pub medium: Medium,
    /// The vendor's id for it.
    pub id: u64,
}

/// One result, imported.
#[derive(Debug)]
pub struct Fetched {
    /// What it was.
    pub candidate: Candidate,
    /// Which file of it was downloaded.
    pub rendition: Rendition,
    /// Whether that file fills the frame asked for without being enlarged.
    pub fills: bool,
    /// The asset it is now — or already was, when the pool held its bytes.
    pub imported: Imported,
}

/// Imports each of `choices` into `project` at `root` as an ordinary
/// `video` or `image` asset, downloading the rendition that fills a
/// `frame` of width×height.
///
/// Each one is decided alone, so one that fails costs none of the others;
/// the answers come back in the order asked. The project is changed in
/// memory only — saving it is the caller's, once, after all of them.
///
/// The file lands as `assets/pixabay-<id>.<ext>`, so where it came from is
/// in its name and nowhere in the format.
pub fn import(
    project: &mut Project,
    root: &Path,
    cache: &Path,
    library: &dyn Library,
    choices: &[Choice],
    frame: (u32, u32),
    probe: &dyn ProbeMedia,
) -> Vec<Result<Fetched, StockError>> {
    choices
        .iter()
        .map(|choice| one(project, root, cache, library, *choice, frame, probe))
        .collect()
}

/// One of [`import`]'s choices.
fn one(
    project: &mut Project,
    root: &Path,
    cache: &Path,
    library: &dyn Library,
    choice: Choice,
    (width, height): (u32, u32),
    probe: &dyn ProbeMedia,
) -> Result<Fetched, StockError> {
    let Choice { medium, id } = choice;
    let candidate = find(cache, library, medium, id)?;
    let rendition = candidate
        .rendition_for(width, height)
        .cloned()
        .ok_or(StockError::NoFile {
            library: library.name(),
            medium,
            id,
        })?;
    let name = format!("pixabay-{id}.{}", extension(&rendition.url, medium));
    let limit = match medium {
        Medium::Video => VIDEO_LIMIT,
        Medium::Image => IMAGE_LIMIT,
    };
    let path = download(
        library,
        &rendition.url,
        limit,
        &cache.join("downloads").join(name),
    )?;
    let imported = import_path(project, root, &path, Some(medium.asset_kind()), probe);
    // Copied into assets/ or refused: either way the download is done with.
    let _ = std::fs::remove_file(&path);
    let imported = imported?
        .imported
        .into_iter()
        .next()
        .ok_or_else(|| StockError::Io {
            path: path.clone(),
            source: std::io::Error::other("the download imported as nothing"),
        })?;
    Ok(Fetched {
        fills: rendition.covers(width, height),
        candidate,
        rendition,
        imported,
    })
}

/// The extension of the file `url` names, lower case — or the medium's usual
/// one when the URL does not say.
fn extension(url: &str, medium: Medium) -> String {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let last = path.rsplit('/').next().unwrap_or_default();
    match last.rsplit_once('.') {
        Some((_, ext))
            if (2..=4).contains(&ext.len()) && ext.chars().all(char::is_alphanumeric) =>
        {
            ext.to_ascii_lowercase()
        }
        _ => String::from(match medium {
            Medium::Video => "mp4",
            Medium::Image => "jpg",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_comes_from_the_url() {
        let video = "https://cdn.pixabay.com/video/2020/05/13/39009-420224623_large.mp4";
        assert_eq!(extension(video, Medium::Video), "mp4");
        let image = "https://pixabay.com/get/TOKEN_1280.PNG?x=1";
        assert_eq!(extension(image, Medium::Image), "png");
        assert_eq!(
            extension("https://pixabay.com/get/abc", Medium::Image),
            "jpg"
        );
    }
}
