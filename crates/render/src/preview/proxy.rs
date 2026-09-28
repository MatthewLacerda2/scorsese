//! Proxies: a small copy of a heavy video, decoded by a preview in place of
//! the original.
//!
//! Decoding a 1080p or 4K screen recording is very often the real cost of a
//! preview frame, not compositing it. A proxy is that source transcoded once,
//! in the background, to a size a reduced-quality preview never outgrows —
//! [`PROXY_SHORT_SIDE`] pixels on its short side, which is exactly the half
//! raster of a 1080p film — and in a shape that is cheap to seek in, since a
//! desktop preview seeks for every frame it shows.
//!
//! **What it keeps is the timing.** The same timestamps, frame for frame
//! (`-fps_mode passthrough`), so a seek to a second in the proxy lands on the
//! frame the same seek lands on in the original, and the `fps` filter a
//! decode puts behind it conforms both identically. What it drops is sound
//! (the mix always reads the original, and decoding sound was never the cost)
//! and every pixel past the short side.
//!
//! **What never gets one:** anything already that small, which would gain
//! nothing, and anything with an alpha channel, or not known to have none —
//! a proxy is H.264, which has no alpha, and a preview that lost a
//! transparent overlay's transparency would be lying about far more than
//! resolution.
//!
//! **Named by the original's hash** ([`file_name`]), plus a version, so a
//! replaced source misses its old proxy by construction and a change to how
//! proxies are made misses every old one. Where they are kept is the caller's
//! choice — the project's rebuildable `cache/` locally ([`folder`]), the
//! server's cache on the web — because both are rebuildable, and nothing
//! decides anything from a proxy but a preview.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use scorsese_core::{Asset, AssetKind, CACHE_DIR, MediaMetadata, Project};

use crate::tools::Tools;

/// The short side of a proxy, in pixels: the half-quality raster of a 1080p
/// film, so a proxy is never enlarged by a preview at half or below.
pub const PROXY_SHORT_SIDE: u32 = 540;

/// Bumped whenever what [`make`] writes changes, so every older proxy is a
/// miss rather than a file somebody made differently.
const VERSION: u32 = 1;

/// The folder under a project's `cache/` a local project's proxies go in.
const FOLDER: &str = "proxies";

/// The proxies a preview may read, by the hash of the source each stands in
/// for.
#[derive(Debug, Clone, Default)]
pub struct Proxies {
    by_hash: HashMap<String, PathBuf>,
}

impl Proxies {
    /// No proxies: every source is read as itself.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads `file` wherever a source hashing to `sha256` is on the timeline.
    pub fn insert(&mut self, sha256: impl Into<String>, file: impl Into<PathBuf>) {
        self.by_hash.insert(sha256.into(), file.into());
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// The proxies of `project`'s assets that are already made in `folder`.
    ///
    /// Only files that exist, so a proxy still being made is simply not used
    /// yet, and the preview reads the original until it is.
    pub fn made_in(folder: &Path, project: &Project) -> Self {
        let mut proxies = Self::new();
        for asset in &project.assets {
            if let Some(sha256) = &asset.sha256 {
                let file = folder.join(file_name(sha256));
                if file.is_file() {
                    proxies.insert(sha256.clone(), file);
                }
            }
        }
        proxies
    }

    /// The file standing in for `asset`, if there is one. Only a moving
    /// picture has one: a still is decoded once per clip, not per frame.
    pub(crate) fn of(&self, asset: &Asset) -> Option<&Path> {
        if !matches!(asset.kind, AssetKind::Video | AssetKind::GeneratedVideo) {
            return None;
        }
        let sha256 = asset.sha256.as_ref()?;
        self.by_hash.get(sha256).map(PathBuf::as_path)
    }
}

/// Whether a source of `kind`, measured as `media`, is worth a proxy: a moving
/// picture, larger than a proxy on its short side, and known to be opaque.
pub fn worth_making(kind: AssetKind, media: &MediaMetadata) -> bool {
    let moving = matches!(kind, AssetKind::Video | AssetKind::GeneratedVideo);
    let large = match (media.width, media.height) {
        (Some(width), Some(height)) => width.min(height) > PROXY_SHORT_SIDE,
        _ => false,
    };
    moving && large && media.has_alpha == Some(false)
}

/// The file name the proxy of a source hashing to `sha256` has, wherever the
/// caller keeps proxies.
pub fn file_name(sha256: &str) -> String {
    format!("{sha256}.proxy{VERSION}.mp4")
}

/// Where a local project at `project_root` keeps its proxies: under its
/// rebuildable `cache/`, which deleting costs nothing but the time to make
/// them again.
pub fn folder(project_root: &Path) -> PathBuf {
    project_root.join(CACHE_DIR).join(FOLDER)
}

/// Why a proxy could not be made.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    /// The file could not be written, or moved into place.
    #[error("writing the proxy {}: {source}", file.display())]
    Io {
        /// The file.
        file: PathBuf,
        /// What the system said.
        source: std::io::Error,
    },
    /// ffmpeg failed to transcode the source.
    #[error("making a proxy of {}: {message}", file.display())]
    Ffmpeg {
        /// The source.
        file: PathBuf,
        /// What ffmpeg said.
        message: String,
    },
}

/// Transcodes `source` into a proxy at `out`, replacing whatever is there.
///
/// Written beside `out` and renamed into place, so a proxy is never read
/// half-made — a preview that checks for the file and finds it can decode it.
///
/// H.264, because the default delivery already requires `libx264`
/// (`docs/output-formats.md`), with a keyframe every fifteen frames and the
/// decoder's cheap path asked for: a desktop preview seeks for every frame it
/// shows, and a seek costs the frames since the last keyframe.
pub fn make(tools: &Tools, source: &Path, out: &Path) -> Result<(), ProxyError> {
    let io = |file: &Path| {
        let file = file.to_path_buf();
        move |source| ProxyError::Io { file, source }
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    let making = out.with_extension("making.mp4");
    let side = PROXY_SHORT_SIDE;
    // The short side brought down to `side` and the long one kept in shape —
    // decided after ffmpeg has turned a phone's rotated picture upright, which
    // is why it is an expression rather than a size worked out here.
    let scale =
        format!("scale='if(gte(iw,ih),-2,min({side},iw))':'if(gte(iw,ih),min({side},ih),-2)'");
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y", "-i"])
        .arg(source)
        .args(["-map", "0:v:0", "-an", "-sn", "-dn", "-vf", &scale])
        .args([
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-tune",
            "fastdecode",
        ])
        .args(["-crf", "26", "-g", "15", "-pix_fmt", "yuv420p"])
        .args(["-fps_mode", "passthrough", "-movflags", "+faststart"])
        .arg(&making)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(io(&making))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&making);
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(ProxyError::Ffmpeg {
            file: source.to_path_buf(),
            message: if message.is_empty() {
                format!("exited with {}", output.status)
            } else {
                message
            },
        });
    }
    std::fs::rename(&making, out).map_err(io(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(width: u32, height: u32, has_alpha: Option<bool>) -> MediaMetadata {
        MediaMetadata {
            width: Some(width),
            height: Some(height),
            has_alpha,
            ..MediaMetadata::default()
        }
    }

    #[test]
    fn only_a_large_opaque_moving_picture_is_worth_a_proxy() {
        let hd = media(1920, 1080, Some(false));
        assert!(worth_making(AssetKind::Video, &hd));
        assert!(worth_making(AssetKind::GeneratedVideo, &hd));
        assert!(worth_making(
            AssetKind::Video,
            &media(1080, 1920, Some(false))
        ));
        assert!(!worth_making(AssetKind::Image, &hd), "a still");
        assert!(
            !worth_making(AssetKind::Video, &media(960, 540, Some(false))),
            "small"
        );
        assert!(
            !worth_making(AssetKind::Video, &media(1920, 1080, Some(true))),
            "alpha"
        );
        assert!(
            !worth_making(AssetKind::Video, &media(1920, 1080, None)),
            "unknown"
        );
    }

    #[test]
    fn a_proxy_is_named_by_its_source_and_the_version_that_made_it() {
        assert_eq!(file_name("abc"), "abc.proxy1.mp4");
        assert_eq!(
            folder(Path::new("film.scor")),
            Path::new("film.scor/cache/proxies")
        );
    }
}
