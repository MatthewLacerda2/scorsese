//! Fixtures that need real ffmpeg, and questions asked of real output.
//!
//! Media is generated here rather than committed: ffmpeg's synthetic sources
//! make exact, tiny files, and generating them through [`Tools`] exercises the
//! command builder every ffmpeg invocation in the workspace must go through.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::{Asset, AssetKind};
use scorsese_render::Tools;

pub(crate) fn tools() -> Tools {
    Tools::discover().expect("ffmpeg and ffprobe must be on PATH to run these tests")
}

/// A fresh empty directory, unique per call.
pub(crate) fn fixture_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-render-{label}-{}-{unique}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    dir
}

/// Runs ffmpeg with a synthetic input to make one small fixture file.
pub(crate) fn generate(tools: &Tools, path: &Path, args: &[&str]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(path)
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Generates a media file inside a project's `assets/` and returns the assets
/// table row pointing at it.
pub(crate) fn generate_asset(
    tools: &Tools,
    project_root: &Path,
    id: &str,
    kind: AssetKind,
    args: &[&str],
) -> Asset {
    let name = format!("{id}.{}", super::extension(kind));
    generate(tools, &project_root.join("assets").join(name), args);
    super::file_asset(id, kind)
}

/// What ffprobe says about the video stream of a rendered file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rendered {
    pub(crate) width: u64,
    pub(crate) height: u64,
    /// Counted by decoding, not read from a header: a container's frame count
    /// is a claim, and this is the number the file actually contains.
    pub(crate) frames: u64,
    /// The rate as ffprobe writes a rational, e.g. `30/1`.
    pub(crate) rate: String,
}

pub(crate) fn inspect(tools: &Tools, file: &Path) -> Rendered {
    let output = tools
        .ffprobe()
        .args(["-v", "error", "-select_streams", "v:0", "-count_frames"])
        .args([
            "-show_entries",
            "stream=width,height,nb_read_frames,r_frame_rate",
        ])
        .args(["-print_format", "json"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    assert!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("ffprobe json parses");
    let stream = &report["streams"][0];
    let number = |key: &str| -> u64 {
        let value = &stream[key];
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            .unwrap_or_else(|| panic!("ffprobe reported no {key}: {stream}"))
    };
    Rendered {
        width: number("width"),
        height: number("height"),
        frames: number("nb_read_frames"),
        rate: stream["r_frame_rate"]
            .as_str()
            .expect("a frame rate")
            .to_owned(),
    }
}

/// The average colour of one frame of a rendered file — enough to tell teal
/// from black without committing a golden image, which is the harness issue's
/// job rather than this one's.
pub(crate) fn mean_rgb(tools: &Tools, file: &Path, frame: u64) -> (u8, u8, u8) {
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-vf", &format!("select=eq(n\\,{frame})")])
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pixels = output.stdout;
    assert!(!pixels.is_empty(), "no frame {frame} in {}", file.display());
    let mean = |offset: usize| -> u8 {
        let total: u64 = pixels
            .iter()
            .skip(offset)
            .step_by(3)
            .map(|&c| u64::from(c))
            .sum();
        (total / (pixels.len() as u64 / 3)) as u8
    };
    (mean(0), mean(1), mean(2))
}
