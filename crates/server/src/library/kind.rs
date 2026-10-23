//! What a library file is: the four kinds a file can be imported as, and the
//! one it cannot.
//!
//! The library held MIDI files too (#678) until #964: once `synth_import` and
//! `synth_export` left the tool list (#785) nothing on the web could use one,
//! so a `.mid` is refused at upload like any other unsupported type, and
//! migration `0025` deleted the rows that held the kind before it went — a
//! stored row whose kind this build could not read would make the whole
//! library unloadable for its owner.
//!
//! A web page (`.html`) is a kind since #1144, because `scorsese import` takes
//! one: it is text, never probed and given no thumbnail, and importing it
//! copies it into the project's `pages/` (`tools::own::catalogue`), where a
//! page lives and is edited — the library item is only where it came from.
//!
//! A document (`.txt`, `.md`) is a kind since #1510 and the one that is
//! **not an asset kind**: a script, a shot list or a briefing the person wrote
//! elsewhere, for the assistant to read (`library_read`), never placed on a
//! timeline or imported. `scorsese import` does not take one, because locally
//! the person's own client reads any file on their disk.

use std::path::Path;

use scorsese_core::AssetKind;
use scorsese_core::pool::infer_kind;
use serde::{Deserialize, Serialize};

use crate::projects::files::MAX_FILE_BYTES;

/// What a library file is. Never whether it was generated — that is the
/// item's brief hash — because a generated shot is a video like any other
/// once it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Moving pictures, with or without sound.
    Video,
    /// A still picture.
    Image,
    /// Sound alone.
    Audio,
    /// A web page the timeline plays (`docs/pages.md`): text, not media.
    Html,
    /// A document for the assistant to read: text, and never an asset.
    Text,
}

/// The extensions a document is uploaded under.
const DOCUMENT: [&str; 2] = ["txt", "md"];

/// The largest document a library takes: 1 MB, far beyond any script.
/// Decimal, as every size scorsese prints is (#1287).
pub const MAX_DOCUMENT_BYTES: usize = 1000 * 1000;

impl Kind {
    /// What a file called `name` is, by the list `scorsese import` uses — so
    /// the web app welcomes exactly the files the CLI does — and a document.
    pub fn of_file_name(name: &str) -> Option<Self> {
        let path = Path::new(name);
        infer_kind(path)
            .and_then(|kind| Self::try_from(kind).ok())
            .or_else(|| {
                let extension = path.extension()?.to_str()?.to_ascii_lowercase();
                DOCUMENT.contains(&extension.as_str()).then_some(Self::Text)
            })
    }

    /// As the database and the API spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Html => "html",
            Self::Text => "text",
        }
    }

    /// What the kind is called in a sentence: "the picture".
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "picture",
            Self::Audio => "sound",
            Self::Html => "page",
            Self::Text => "document",
        }
    }

    /// The asset kind a file of this kind is imported as; `None` for a
    /// document, which is never imported.
    pub fn asset_kind(self) -> Option<AssetKind> {
        match self {
            Self::Video => Some(AssetKind::Video),
            Self::Image => Some(AssetKind::Image),
            Self::Audio => Some(AssetKind::Audio),
            Self::Html => Some(AssetKind::Html),
            Self::Text => None,
        }
    }

    /// The extension a generation's output of this kind is kept under: an
    /// ElevenLabs line is MP3, a Gemini still JPEG, a Veo shot MP4. Nothing
    /// generates a page or a document.
    pub fn generated_extension(self) -> &'static str {
        match self {
            Self::Audio => "mp3",
            Self::Image => "jpg",
            Self::Video | Self::Html | Self::Text => "mp4",
        }
    }

    /// Whether a file of this kind is media, probed by ffprobe and drawn as a
    /// thumbnail. A page and a document are neither.
    pub fn is_media(self) -> bool {
        matches!(self, Self::Video | Self::Image | Self::Audio)
    }

    /// The most bytes a file of this kind may hold, for the kinds held to
    /// being text — `None` for media, which only the upload cap limits. A
    /// page is held to what a project keeps among its pages.
    pub fn max_text_bytes(self) -> Option<usize> {
        match self {
            Self::Html => Some(MAX_FILE_BYTES),
            Self::Text => Some(MAX_DOCUMENT_BYTES),
            Self::Video | Self::Image | Self::Audio => None,
        }
    }

    /// The `Content-Type` a file of this kind with `extension` is served as.
    ///
    /// Only for the extensions [`Kind::of_file_name`] accepts; anything else
    /// is served as bytes, which a browser will not try to run. A page is
    /// served as plain text, never `text/html`: the library is served from
    /// the app's own origin, and a page run there could act as its owner. A
    /// document is plain text too, Markdown included, so a browser shows it
    /// as written.
    pub fn content_type(self, extension: &str) -> &'static str {
        let known = match (self, extension) {
            (Self::Video, "mp4") => "video/mp4",
            (Self::Video, "m4v") => "video/x-m4v",
            (Self::Video, "mov") => "video/quicktime",
            (Self::Video, "webm") => "video/webm",
            (Self::Video, "mkv") => "video/x-matroska",
            (Self::Video, "avi") => "video/x-msvideo",
            (Self::Video, "mpg" | "mpeg") => "video/mpeg",
            (Self::Video, "wmv") => "video/x-ms-wmv",
            (Self::Image, "png") => "image/png",
            (Self::Image, "jpg" | "jpeg") => "image/jpeg",
            (Self::Image, "webp") => "image/webp",
            (Self::Image, "gif") => "image/gif",
            (Self::Image, "bmp") => "image/bmp",
            (Self::Image, "tif" | "tiff") => "image/tiff",
            (Self::Image, "avif") => "image/avif",
            (Self::Audio, "mp3") => "audio/mpeg",
            (Self::Audio, "wav") => "audio/wav",
            (Self::Audio, "m4a") => "audio/mp4",
            (Self::Audio, "aac") => "audio/aac",
            (Self::Audio, "flac") => "audio/flac",
            (Self::Audio, "ogg") => "audio/ogg",
            (Self::Audio, "opus") => "audio/opus",
            (Self::Audio, "aiff") => "audio/aiff",
            (Self::Audio, "wma") => "audio/x-ms-wma",
            (Self::Html, "html") | (Self::Text, "txt" | "md") => "text/plain; charset=utf-8",
            _ => "",
        };
        if known.is_empty() {
            "application/octet-stream"
        } else {
            known
        }
    }
}

impl TryFrom<AssetKind> for Kind {
    type Error = AssetKind;

    fn try_from(kind: AssetKind) -> Result<Self, AssetKind> {
        match kind {
            AssetKind::Video => Ok(Self::Video),
            AssetKind::Image => Ok(Self::Image),
            AssetKind::Audio => Ok(Self::Audio),
            AssetKind::Html => Ok(Self::Html),
            other => Err(other),
        }
    }
}

impl TryFrom<String> for Kind {
    type Error = String;

    fn try_from(kind: String) -> Result<Self, String> {
        match kind.as_str() {
            "video" => Ok(Self::Video),
            "image" => Ok(Self::Image),
            "audio" => Ok(Self::Audio),
            "html" => Ok(Self::Html),
            "text" => Ok(Self::Text),
            _ => Err(format!("{kind:?} is not a library kind")),
        }
    }
}
