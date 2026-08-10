//! Turning ffprobe's JSON into the typed metadata the model stores.
//!
//! ffprobe reports numbers as strings and frame rates as `"30000/1001"`, and
//! omits fields freely. Everything awkward about that shape is confined to
//! this file so no consumer ever re-parses it.

use scorsese_core::{Fps, MediaMetadata};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Report {
    #[serde(default)]
    format: Format,
    #[serde(default)]
    streams: Vec<Stream>,
}

#[derive(Debug, Default, Deserialize)]
struct Format {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Stream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    duration: Option<String>,
    channels: Option<u16>,
    sample_rate: Option<String>,
}

impl Report {
    pub(super) fn into_metadata(self) -> MediaMetadata {
        let video = self.stream("video");
        let audio = self.stream("audio");

        MediaMetadata {
            duration_seconds: self
                .format
                .duration
                .as_deref()
                .and_then(parse_number)
                .or_else(|| {
                    video
                        .and_then(|s| s.duration.as_deref())
                        .and_then(parse_number)
                }),
            width: video.and_then(|stream| stream.width),
            height: video.and_then(|stream| stream.height),
            frame_rate: video
                .and_then(|stream| stream.r_frame_rate.as_deref())
                .and_then(parse_rate),
            has_alpha: video
                .and_then(|stream| stream.pix_fmt.as_deref())
                .map(carries_alpha),
            audio_channels: audio.and_then(|stream| stream.channels),
            sample_rate: audio
                .and_then(|stream| stream.sample_rate.as_deref())
                .and_then(parse_number)
                .map(|rate| rate as u32),
        }
    }

    fn stream(&self, kind: &str) -> Option<&Stream> {
        self.streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some(kind))
    }
}

fn parse_number(raw: &str) -> Option<f64> {
    raw.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

/// Whether a pixel format carries transparency, decided from ffmpeg's own name
/// for it.
///
/// ffprobe reports the format and nothing about what is in it, so the name is
/// the only evidence there is. It is a reliable one: ffmpeg spells an alpha
/// plane into the name in a small number of ways, and every one of them is a
/// prefix — `rgba64le`, `yuva420p10le` and `gbrap12be` all announce themselves
/// in their first four characters. Matching on prefixes rather than "contains
/// an `a`" is what keeps `gray` and `bayer_bggr8` out. The list covers every
/// four-component format `ffmpeg -pix_fmts` lists, plus the two-component
/// `ya8`/`ya16` pair, which is grey with an alpha plane.
///
/// `pal8` is here on purpose, and it is the one entry that is not a statement
/// about the pixel format: a palette image keeps its transparency in the
/// palette, where the format name cannot show it. Some palette images are
/// fully opaque, so this costs those an unnecessary pair of filters — which is
/// the cheap way to be wrong, against a visible rim on the ones that are not.
///
/// Anything unrecognised is treated as opaque. That is the conservative answer
/// too: it is what the decode chain did before this fact was recorded at all.
fn carries_alpha(pix_fmt: &str) -> bool {
    const ALPHA: [&str; 11] = [
        "rgba", "bgra", "argb", "abgr", "yuva", "ayuv", "vuya", "uyva", "gbra", "ya", "pal8",
    ];
    ALPHA.iter().any(|marker| pix_fmt.starts_with(marker))
}

/// ffprobe's `"30000/1001"` is already the shape [`Fps`] holds, so it is kept
/// exactly rather than collapsed to 29.97. A still image reports `"0/0"`,
/// which is not a frame rate and becomes `None`.
fn parse_rate(raw: &str) -> Option<Fps> {
    raw.parse().ok()
}
