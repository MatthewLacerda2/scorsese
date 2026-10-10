//! Tracing a project's picture: finding it, decoding it, and writing the SVG
//! beside the pages.
//!
//! The SVG is not an asset. A page loads it and draws it, and the page is what
//! goes on the timeline — the way a Lottie `stock_import` keeps is a file
//! under `pages/` rather than a clip. It is written at `pages/<name>.svg`,
//! replacing what was there, so tracing again with other choices is how a
//! tracing is adjusted.

use std::path::{Path, PathBuf};

use scorsese_compositor::Resolution;
use scorsese_core::probe::ProbeMedia;
use scorsese_core::{AssetId, AssetKind, GenerationState, PAGES_DIR, Project};

use super::{Traced, Tracing, trace};
use crate::frames::{FrameError, raw_frame};
use crate::probe::Ffprobe;
use crate::tools::Tools;

/// A picture larger than this across or down is traced at this size. Twice a
/// generated image's own, and more than any drawing needs on a 1080p board.
const LARGEST: u32 = 2048;

/// Why a picture could not be traced. Nothing is written by a refusal.
#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    /// No asset has the id.
    #[error("no asset is called `{0}`")]
    Unknown(AssetId),
    /// The asset is not a picture.
    #[error("`{0}` is not a picture: only an image or a generated_image traces")]
    NotAPicture(AssetId),
    /// A generated picture that has not been generated.
    #[error("`{0}` has not been generated yet — generate it first, then trace it")]
    NotGenerated(AssetId),
    /// The name cannot be a file name under `pages/`.
    #[error("`{0}` cannot name the drawing: use letters, digits, `-` and `_`")]
    BadName(String),
    /// The picture's size could not be read.
    #[error("reading {}: {message}", file.display())]
    Unreadable {
        /// The picture.
        file: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// ffmpeg could not decode it.
    #[error(transparent)]
    Decode(#[from] FrameError),
    /// The SVG could not be written.
    #[error("cannot write {}: {source}", file.display())]
    Unwritable {
        /// The file.
        file: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}

/// What [`vectorize`] wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Vectorized {
    /// The drawing.
    pub traced: Traced,
    /// Where it is, relative to the project root: `pages/<name>.svg`.
    pub path: String,
}

/// Traces the picture `asset` into `pages/<name>.svg`.
pub fn vectorize(
    tools: &Tools,
    project: &Project,
    project_root: &Path,
    asset: &str,
    name: &str,
    tracing: Tracing,
) -> Result<Vectorized, TraceError> {
    let id = AssetId::new(asset);
    let found = project
        .asset(&id)
        .ok_or_else(|| TraceError::Unknown(id.clone()))?;
    if !matches!(found.kind, AssetKind::Image | AssetKind::GeneratedImage) {
        return Err(TraceError::NotAPicture(id));
    }
    let generated =
        found.kind != AssetKind::GeneratedImage || found.state == Some(GenerationState::Generated);
    let Some(path) = found.path.as_ref().filter(|_| generated) else {
        return Err(TraceError::NotGenerated(id));
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(TraceError::BadName(name.to_owned()));
    }
    let (rgba, width, height) = read(tools, &path.resolve(project_root))?;
    let traced = trace(&rgba, width, height, tracing, name);
    let relative = format!("{PAGES_DIR}/{name}.svg");
    let file = project_root.join(&relative);
    let written = std::fs::create_dir_all(project_root.join(PAGES_DIR))
        .and_then(|()| scorsese_core::write::atomically(&file, &traced.svg));
    written.map_err(|source| TraceError::Unwritable { file, source })?;
    Ok(Vectorized {
        traced,
        path: relative,
    })
}

/// A picture file as RGBA pixels, four bytes each, with its width and height —
/// no larger than [`LARGEST`] either way. Any still ffmpeg reads: PNG, JPEG,
/// WebP.
pub fn read(tools: &Tools, file: &Path) -> Result<(Vec<u8>, usize, usize), TraceError> {
    let unreadable = |message: String| TraceError::Unreadable {
        file: file.to_path_buf(),
        message,
    };
    let media = Ffprobe::new(tools.clone())
        .probe(file)
        .map_err(|error| unreadable(error.to_string()))?;
    let (Some(width), Some(height)) = (media.width, media.height) else {
        return Err(unreadable("it has no picture in it".to_owned()));
    };
    let scale = f64::from(LARGEST) / f64::from(width.max(height));
    let (width, height) = if scale < 1.0 {
        (shrunk(width, scale), shrunk(height, scale))
    } else {
        (width, height)
    };
    let size = Resolution::new(width, height).map_err(|error| unreadable(error.to_string()))?;
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(file)
        .args(["-frames:v", "1", "-vf", &format!("scale={width}:{height}")]);
    let frame = raw_frame(command, file, size)?;
    Ok((frame.bytes().to_vec(), width as usize, height as usize))
}

/// A side scaled down, kept even and at least two pixels.
fn shrunk(side: u32, scale: f64) -> u32 {
    let scaled = (f64::from(side) * scale).round();
    (scaled as u32 / 2 * 2).max(2)
}
