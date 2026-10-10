//! What a render is asked to be, and the key it is kept under.
//!
//! [`Ask`] is what a request says, every field optional; [`Settings`] is what
//! it means once every default is filled in — the form stored with the row
//! and hashed into its key, so `{}` and `{"container": "mp4"}` are one render.

use scorsese_core::{Project, hash_bytes};
use scorsese_render::{
    AudioCodec, Bands, Container, LoudnessTarget, OutputFormat, Quality, RenderSettings,
    Resolution, VideoCodec,
};
use serde::{Deserialize, Serialize};

/// A request for a render: the choices `scorsese render` and the MCP `render`
/// tool offer about the delivered file, spelled as the MCP tool spells them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ask {
    /// `mp4`, `mkv`, `avi` or `wmv`; `mp3`, `wav` or `m4a` for sound only.
    /// Defaults to mp4.
    #[serde(default)]
    pub container: Option<String>,
    /// The picture codec; defaults to the container's.
    #[serde(default)]
    pub video_codec: Option<String>,
    /// The sound codec; defaults to the container's.
    #[serde(default)]
    pub audio_codec: Option<String>,
    /// `WIDTHxHEIGHT`; defaults to 1920x1080, or over HTTP to the size of the
    /// platform the project is made for (#1016). Refused for sound only.
    #[serde(default)]
    pub resolution: Option<String>,
    /// `false` leaves out the band an ungenerated narration line draws across
    /// the foot of the picture (#966, #983); defaults to `true`, every band
    /// drawn. Refused for sound only.
    #[serde(default)]
    pub narration_bands: Option<bool>,
    /// The integrated loudness to deliver the soundtrack at, in LUFS, between
    /// -40 and -5 (#968, #990); without it the mix is delivered as balanced.
    #[serde(default)]
    pub loudness: Option<f64>,
}

/// A render's settings with every default filled in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// The container, by name.
    pub container: String,
    /// The picture codec, or `None` for sound only.
    pub video_codec: Option<String>,
    /// The sound codec.
    pub audio_codec: String,
    /// The picture's size, or `None` for sound only.
    pub resolution: Option<String>,
    /// The preview quality this was drawn at, when it is a **preview** (#542)
    /// rather than a finished render — `None` for every render a person
    /// downloads. Left out of the stored form when absent, so a finished
    /// render's key is what it was before previews existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Whether an ungenerated narration line draws its slug band. Left out of
    /// the stored form when it does, the default — so every render asked for
    /// before the choice existed keeps its key, and is not rendered again.
    #[serde(default = "drawn", skip_serializing_if = "is_drawn")]
    pub narration_bands: bool,
    /// The loudness target in LUFS, or `None` to deliver the mix as balanced.
    /// Left out of the stored form when absent, so every render asked for
    /// before the choice existed keeps its key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness: Option<f64>,
}

/// [`Settings::narration_bands`]' default: every band drawn.
fn drawn() -> bool {
    true
}

/// Whether `narration_bands` is the default, and so left out of the key.
fn is_drawn(narration_bands: &bool) -> bool {
    *narration_bands
}

/// A request for a preview of the cut (#542): the delivery size it previews,
/// and the quality to draw it at.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewAsk {
    /// The size the film would be delivered at, `WIDTHxHEIGHT`: the shape,
    /// and what the quality is a fraction of. Defaults to 1920x1080, or over
    /// HTTP to the size of the platform the project is made for (#1016).
    #[serde(default)]
    pub resolution: Option<String>,
    /// `full`, `half` or `quarter`; defaults to half.
    #[serde(default)]
    pub quality: Option<String>,
}

impl Settings {
    /// What `ask` means, or why `docs/output-formats.md` does not allow it —
    /// in the words the CLI refuses it in, since it is the same constructor.
    pub fn from_ask(ask: &Ask) -> Result<Self, String> {
        let Parsed {
            format,
            resolution,
            bands,
            loudness,
        } = parse(ask)?;
        Ok(Self {
            container: format.container().name().to_owned(),
            video_codec: format.video().map(|codec| codec.name().to_owned()),
            audio_codec: format.audio().name().to_owned(),
            resolution: resolution.map(|resolution| resolution.to_string()),
            preview: None,
            narration_bands: bands == Bands::Drawn,
            loudness: loudness.map(LoudnessTarget::lufs),
        })
    }

    /// What a preview request means: an mp4 at the quality's fraction of the
    /// delivery size, marked as a preview so it is keyed, queued and listed
    /// apart from finished renders.
    pub fn from_preview(ask: &PreviewAsk) -> Result<Self, String> {
        let quality = match &ask.quality {
            Some(name) => name.parse::<Quality>().map_err(|e| e.to_string())?,
            None => Quality::default(),
        };
        let full = match &ask.resolution {
            Some(text) => text
                .parse::<Resolution>()
                .map_err(|e| format!("resolution: {e}"))?,
            None => Resolution::HD,
        };
        Ok(Self {
            preview: Some(quality.name().to_owned()),
            ..Self::from_ask(&Ask {
                resolution: Some(quality.raster(full).to_string()),
                ..Ask::default()
            })?
        })
    }

    /// The quality this preview is drawn at, or `None` for a finished render.
    pub fn quality(&self) -> Result<Option<Quality>, String> {
        self.preview
            .as_deref()
            .map(|name| name.parse::<Quality>().map_err(|e| e.to_string()))
            .transpose()
    }

    /// The render settings these describe, at `project`'s own frame rate.
    ///
    /// Parsed again from their names, so settings that did not come from
    /// [`Settings::from_ask`] — a job row edited by hand — are refused rather
    /// than trusted.
    pub fn render(&self, project: &Project) -> Result<RenderSettings, String> {
        let parsed = parse(&Ask {
            container: Some(self.container.clone()),
            video_codec: self.video_codec.clone(),
            audio_codec: Some(self.audio_codec.clone()),
            resolution: self.resolution.clone(),
            narration_bands: Some(self.narration_bands),
            loudness: self.loudness,
        })?;
        let resolution = parsed.resolution.unwrap_or(Resolution::HD);
        Ok(RenderSettings::new(resolution, project.timeline_fps)
            .with_format(parsed.format)
            .with_bands(parsed.bands)
            .with_loudness(parsed.loudness))
    }

    /// The delivered file's extension: the container's name (`docs/output-formats.md`).
    pub fn extension(&self) -> &str {
        &self.container
    }

    /// The delivered file's `Content-Type`.
    pub fn content_type(&self) -> &'static str {
        match self.container.as_str() {
            "mp4" => "video/mp4",
            "mkv" => "video/x-matroska",
            "avi" => "video/x-msvideo",
            "wmv" => "video/x-ms-wmv",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "m4a" => "audio/mp4",
            _ => "application/octet-stream",
        }
    }
}

/// What an [`Ask`] means to the renderer.
struct Parsed {
    /// The format it names.
    format: OutputFormat,
    /// Its picture's size — `None` for sound only.
    resolution: Option<Resolution>,
    /// Whether narration bands are drawn.
    bands: Bands,
    /// The loudness to deliver at, if any.
    loudness: Option<LoudnessTarget>,
}

/// What `ask` means, or why it is refused.
fn parse(ask: &Ask) -> Result<Parsed, String> {
    let container = match &ask.container {
        Some(name) => name.parse::<Container>().map_err(|e| e.to_string())?,
        None => Container::Mp4,
    };
    let video = ask.video_codec.as_deref().map(str::parse::<VideoCodec>);
    let video = video.transpose().map_err(|e| e.to_string())?;
    let audio = ask.audio_codec.as_deref().map(str::parse::<AudioCodec>);
    let audio = audio.transpose().map_err(|e| e.to_string())?;
    let format = OutputFormat::new(container, video, audio).map_err(|e| e.to_string())?;
    let resolution = match &ask.resolution {
        Some(text) => {
            format
                .picture_setting("a resolution")
                .map_err(|e| e.to_string())?;
            Some(text.parse().map_err(|e| format!("resolution: {e}"))?)
        }
        None => format.has_picture().then_some(Resolution::HD),
    };
    // Refused for a format with no picture in the stdio tool's and the CLI's
    // words.
    let bands = match ask.narration_bands {
        Some(false) => {
            format
                .picture_setting("leaving narration bands out")
                .map_err(|e| e.to_string())?;
            Bands::Omitted
        }
        Some(true) | None => Bands::Drawn,
    };
    // The CLI's `--loudness` and the stdio tool's `loudness`, held to the
    // same range in the same words.
    let loudness = ask.loudness.map(LoudnessTarget::new).transpose();
    let loudness = loudness.map_err(|e| e.to_string())?;
    Ok(Parsed {
        format,
        resolution,
        bands,
        loudness,
    })
}

/// The key a render of `project` with `settings` is kept under: a SHA-256 of
/// the two, serialised, behind a label naming what is hashed.
///
/// The document is serialised by `serde` from the loaded [`Project`], which
/// keeps no maps with an order of their own, so one document is one key
/// however its JSON was spelled when it was saved.
pub fn key(project: &Project, settings: &Settings) -> Result<String, serde_json::Error> {
    let mut bytes = b"scorsese render: document, settings\n".to_vec();
    bytes.extend(serde_json::to_vec(project)?);
    bytes.push(b'\n');
    bytes.extend(serde_json::to_vec(settings)?);
    Ok(hash_bytes(&bytes))
}
