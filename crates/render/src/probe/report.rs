//! Turning ffprobe's JSON into the typed metadata the model stores.
//!
//! ffprobe reports numbers as strings and frame rates as `"30000/1001"`, and
//! omits fields freely. Everything awkward about that shape is confined to
//! this file so no consumer ever re-parses it.

use scorsese_core::asset::MediaMetadata;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Report {
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
    duration: Option<String>,
    channels: Option<u16>,
    sample_rate: Option<String>,
}

impl Report {
    pub fn into_metadata(self) -> MediaMetadata {
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

/// `"30000/1001"` becomes 29.97. A still image reports `"0/0"`, which is not
/// a frame rate and becomes `None` rather than a division by zero.
fn parse_rate(raw: &str) -> Option<f64> {
    let (numerator, denominator) = raw.split_once('/')?;
    let numerator: f64 = numerator.parse().ok()?;
    let denominator: f64 = denominator.parse().ok()?;
    if denominator == 0.0 || numerator == 0.0 {
        return None;
    }
    Some(numerator / denominator)
}
