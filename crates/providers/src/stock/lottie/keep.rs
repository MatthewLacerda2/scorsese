//! Importing a Lottie: its JSON written under `pages/`, for a page to play.
//!
//! **Not an asset.** A Lottie is not placed on the timeline by itself: an
//! `html` clip's page loads it and draws it with the shipped lottie-web, so
//! it needs no asset kind and no format change. It lands **beside the pages**
//! (`pages/lottie-<id>.json`, loaded from a page as `lottie-<id>.json`)
//! rather than in `assets/`, because on the web `assets/` holds only library
//! media an asset names, and a stored project keeps every file under
//! `pages/` (#777) — so the same import works in a `.scor` folder and on the
//! web, with nothing on the server to know about it.

use std::path::Path;

use serde::Deserialize;

use scorsese_core::PAGES_DIR;

use super::super::candidate::{Candidate, Medium};
use super::super::library::{Library, StockError};
use super::super::search::find;

/// The most a Lottie may be. Most are tens or hundreds of kilobytes; one
/// with many pictures embedded can reach megabytes.
const LIMIT: u64 = 32 * 1024 * 1024;

/// One animation, imported.
#[derive(Debug, Clone, PartialEq)]
pub struct Kept {
    /// What it was.
    pub candidate: Candidate,
    /// Where it was written, from the project root: `pages/lottie-<id>.json`.
    pub path: String,
    /// What a page loads it as, from beside it: `lottie-<id>.json`.
    pub file: String,
    /// Whether the same file was already there, so nothing was written.
    pub reused: bool,
    /// Its own width and height, in its own units — what lottie-web scales
    /// to the box it is drawn in.
    pub size: (f64, f64),
    /// Frames a second it was authored at.
    pub fps: f64,
    /// How many frames it runs: its out point less its in point.
    pub frames: f64,
    /// Bytes.
    pub bytes: u64,
    /// Pictures it names outside itself, which a page would look for beside
    /// it and not find. Most Lottie files embed theirs.
    pub outside: Vec<String>,
}

impl Kept {
    /// How long it runs once, in seconds.
    pub fn seconds(&self) -> f64 {
        if self.fps > 0.0 {
            self.frames / self.fps
        } else {
            0.0
        }
    }
}

/// What is read of a Lottie file to know it is one: the fields every player
/// needs. The version and the layers are only required, never read.
#[derive(Debug, Deserialize)]
struct Shape {
    /// The format version.
    #[serde(rename = "v")]
    _version: String,
    /// Frames a second.
    fr: f64,
    /// In point, in frames.
    ip: f64,
    /// Out point, in frames.
    op: f64,
    /// Width.
    w: f64,
    /// Height.
    h: f64,
    /// Its layers.
    #[serde(rename = "layers")]
    _layers: Vec<serde::de::IgnoredAny>,
    /// Pictures and precompositions it uses.
    #[serde(default)]
    assets: Vec<Picture>,
}

/// One entry of a Lottie's `assets`: a picture when it has a `p`.
#[derive(Debug, Deserialize)]
struct Picture {
    /// The folder the picture is in.
    #[serde(default)]
    u: Option<String>,
    /// The picture's file name, or the picture itself as a `data:` URL.
    #[serde(default)]
    p: Option<String>,
    /// `1` when `p` is the picture itself.
    #[serde(default)]
    e: Option<u8>,
}

/// Imports each of `ids` into the project at `root` as
/// `pages/lottie-<id>.json`, each decided alone — one that fails costs none
/// of the others — in the order asked.
pub fn keep(
    root: &Path,
    cache: &Path,
    library: &dyn Library,
    ids: &[u64],
) -> Vec<Result<Kept, StockError>> {
    ids.iter()
        .map(|&id| one(root, cache, library, id))
        .collect()
}

/// One of [`keep`]'s.
fn one(root: &Path, cache: &Path, library: &dyn Library, id: u64) -> Result<Kept, StockError> {
    let candidate = find(cache, library, Medium::Lottie, id)?;
    let json = candidate.largest().cloned().ok_or(StockError::NoFile {
        library: library.name(),
        medium: Medium::Lottie,
        id,
    })?;
    let mut bytes = Vec::new();
    library.download(&json.url, LIMIT, &mut bytes)?;
    let shape: Shape = serde_json::from_slice(&bytes).map_err(|error| StockError::NotLottie {
        library: library.name(),
        id,
        said: error.to_string(),
    })?;
    let file = format!("lottie-{id}.json");
    let path = format!("{PAGES_DIR}/{file}");
    let target = root.join(&path);
    let reused = std::fs::read(&target).is_ok_and(|there| there == bytes);
    if !reused {
        let io = |source| StockError::Io {
            path: target.clone(),
            source,
        };
        std::fs::create_dir_all(root.join(PAGES_DIR)).map_err(io)?;
        scorsese_core::write::atomically(&target, &bytes).map_err(io)?;
    }
    Ok(Kept {
        outside: outside(&shape),
        size: (shape.w, shape.h),
        fps: shape.fr,
        frames: (shape.op - shape.ip).max(0.0),
        bytes: bytes.len() as u64,
        candidate,
        path,
        file,
        reused,
    })
}

/// The pictures `shape` names by file rather than carrying.
fn outside(shape: &Shape) -> Vec<String> {
    shape
        .assets
        .iter()
        .filter(|picture| picture.e != Some(1))
        .filter_map(|picture| {
            let file = picture.p.as_deref().filter(|p| !p.is_empty())?;
            (!file.starts_with("data:"))
                .then(|| format!("{}{file}", picture.u.as_deref().unwrap_or_default()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(assets: serde_json::Value) -> Shape {
        serde_json::from_value(serde_json::json!({
            "v": "5.7.0", "fr": 30, "ip": 0, "op": 60, "w": 100, "h": 100,
            "layers": [], "assets": assets
        }))
        .unwrap()
    }

    #[test]
    fn only_a_picture_named_by_file_is_outside() {
        let named = shape(serde_json::json!([
            { "id": "a", "u": "images/", "p": "img_0.png", "e": 0 },
            { "id": "b", "u": "", "p": "data:image/png;base64,AAAA", "e": 1 },
            { "id": "c", "layers": [] },
        ]));
        assert_eq!(outside(&named), ["images/img_0.png"]);
        assert!(outside(&shape(serde_json::json!([]))).is_empty());
    }

    #[test]
    fn a_file_that_is_not_a_lottie_does_not_read_as_one() {
        assert!(serde_json::from_str::<Shape>(r#"{"hello": "world"}"#).is_err());
    }
}
