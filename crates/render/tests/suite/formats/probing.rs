//! What a delivered file says it is, asked of the file rather than of us.

use std::path::Path;

use scorsese_render::Tools;

/// The shape of one delivered file, as ffprobe reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delivered {
    /// ffprobe's demuxer name, which is a list — one demuxer reads several
    /// containers, so `mp4` comes back as `mov,mp4,m4a,3gp,3g2,mj2`. Compared
    /// with [`Delivered::is`] rather than by equality.
    pub(crate) container: String,
    /// The codec of the first video stream.
    pub(crate) video: String,
    /// The codec of the first audio stream. Empty when the file is silent,
    /// which for these fixtures would itself be a failure.
    pub(crate) audio: String,
}

impl Delivered {
    /// Whether `demuxer` is one of the containers ffprobe named — membership
    /// of the list, not a substring of it.
    pub(crate) fn is(&self, demuxer: &str) -> bool {
        self.container.split(',').any(|name| name == demuxer)
    }
}

/// Asks ffprobe what container and codecs a file holds.
pub(crate) fn probe(tools: &Tools, file: &Path) -> Delivered {
    let output = tools
        .ffprobe()
        .args(["-v", "error", "-show_entries"])
        .args(["format=format_name:stream=codec_type,codec_name"])
        .args(["-print_format", "json"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    assert!(
        output.status.success(),
        "ffprobe failed on {}: {}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("ffprobe json parses");
    let codec = |kind: &str| -> String {
        report["streams"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .find(|stream| stream["codec_type"] == kind)
            .and_then(|stream| stream["codec_name"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    Delivered {
        container: report["format"]["format_name"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        video: codec("video"),
        audio: codec("audio"),
    }
}
