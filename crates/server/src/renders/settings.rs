//! What a render is asked to be, and the key it is kept under.
//!
//! [`Ask`] is what a request says, every field optional; [`Settings`] is what
//! it means once every default is filled in — the form stored with the row
//! and hashed into its key, so `{}` and `{"container": "mp4"}` are one render.

use scorsese_core::{Project, Ratio, hash_bytes};
use scorsese_render::{
    Bands, FrameRange, LoudnessTarget, OutputFormat, Quality, RenderAsk, RenderSettings,
    Resolution, Spelled,
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
    /// `WIDTHxHEIGHT`, or a shape like `9:16` at delivery size; defaults to
    /// 1920x1080, or over HTTP to the project's own frame at delivery size
    /// (#1278). Refused for sound only, and beside `platform`.
    #[serde(default)]
    pub resolution: Option<String>,
    /// The placement the file is made for, which sizes it — `tiktok`,
    /// `youtube`… (`scorsese_core::style::Platform`). Refused for sound only,
    /// and beside `resolution`.
    #[serde(default)]
    pub platform: Option<String>,
    /// The frame rate to deliver at: `30`, or a fraction like `30000/1001`.
    /// Defaults to the project's own timeline rate. Refused for sound only.
    #[serde(default)]
    pub fps: Option<String>,
    /// The stretch of the timeline to render, in frames: `30:120`, `30:` or
    /// `:120`, as `scorsese render --range` reads it. Defaults to the whole
    /// timeline.
    #[serde(default)]
    pub range: Option<String>,
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
    /// The picture's size, or `None` for sound only. A platform asked for is
    /// kept as the size it delivers at: the file is the same either way.
    pub resolution: Option<String>,
    /// The delivery rate, or `None` for the project's own. Left out of the
    /// stored form when absent, so every render asked for before the choice
    /// existed keeps its key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<String>,
    /// The frames rendered, as `FrameRange` writes them, or `None` for the
    /// whole timeline. Left out of the stored form when absent, likewise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
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
    /// HTTP to the project's own frame at delivery size (#1278).
    #[serde(default)]
    pub resolution: Option<String>,
    /// `full`, `half` or `quarter`; defaults to half.
    #[serde(default)]
    pub quality: Option<String>,
}

impl Ask {
    /// Whether this asks for a picture without saying its size, so a surface
    /// that has the project open fills in its own frame (#1278) — never when
    /// a platform sizes it, which a resolution beside would contradict.
    pub fn leaves_size(&self, settings: &Settings) -> bool {
        self.resolution.is_none() && self.platform.is_none() && settings.resolution.is_some()
    }
}

impl Settings {
    /// What `ask` means, or why `docs/output-formats.md` does not allow it —
    /// in the words the CLI refuses it in, since it is the same constructor.
    pub fn from_ask(ask: &Ask) -> Result<Self, String> {
        let range = ask.range.as_deref().map(frames).transpose()?;
        let (ask, format) = checked(ask)?;
        Ok(Self {
            container: format.container().name().to_owned(),
            video_codec: format.video().map(|codec| codec.name().to_owned()),
            audio_codec: format.audio().name().to_owned(),
            // No project is open here: a size left out over HTTP was already
            // filled with the project's own frame, so this fallback is 16:9's.
            resolution: format
                .has_picture()
                .then(|| ask.resolution(Ratio::WIDE).to_string()),
            fps: ask.fps.map(|fps| fps.to_string()),
            range: range.map(|range| range.to_string()),
            preview: None,
            narration_bands: ask.bands == Bands::Drawn,
            loudness: ask.loudness.map(LoudnessTarget::lufs),
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
        let (ask, format) = checked(&Ask {
            container: Some(self.container.clone()),
            video_codec: self.video_codec.clone(),
            audio_codec: Some(self.audio_codec.clone()),
            resolution: self.resolution.clone(),
            platform: None,
            fps: self.fps.clone(),
            range: None,
            narration_bands: Some(self.narration_bands),
            loudness: self.loudness,
        })?;
        ask.settings(format, project).map_err(|e| e.to_string())
    }

    /// The frames to render, parsed again from the stored form like
    /// [`Settings::render`]'s settings.
    pub fn range(&self) -> Result<FrameRange, String> {
        self.range.as_deref().map_or(Ok(FrameRange::ALL), frames)
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

/// What `ask` means and the format it is delivered in, or why it is refused
/// — read and checked by the rules every surface shares ([`RenderAsk`]), so
/// in the words the CLI and the stdio tool refuse it in. The container
/// defaults to mp4 rather than to a path's, since the server names its own
/// file.
fn checked(ask: &Ask) -> Result<(RenderAsk, OutputFormat), String> {
    let ask = RenderAsk::read(&Spelled {
        container: ask.container.as_deref(),
        video_codec: ask.video_codec.as_deref(),
        audio_codec: ask.audio_codec.as_deref(),
        resolution: ask.resolution.as_deref(),
        platform: ask.platform.as_deref(),
        fps: ask.fps.as_deref(),
        narration_bands: ask.narration_bands,
        loudness: ask.loudness,
        ..Spelled::default()
    })
    .map_err(|e| e.to_string())?;
    let format = ask.format().map_err(|e| e.to_string())?;
    ask.check(format).map_err(|e| e.to_string())?;
    Ok((ask, format))
}

/// `text` read as a frame range by `FrameRange`'s own rules — the ones
/// `scorsese render --range` and the MCP `render` tool read it by — its
/// refusal named for the argument as theirs is.
fn frames(text: &str) -> Result<FrameRange, String> {
    text.parse().map_err(|problem| format!("range: {problem}"))
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
