//! What a library file is: the three kinds a file can be imported as.

use std::path::Path;

use scorsese_core::AssetKind;
use scorsese_core::pool::infer_kind;
use serde::{Deserialize, Serialize};

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
}

impl Kind {
    /// What a file called `name` is, by the list `scorsese import` uses — so
    /// the web app welcomes exactly the files the CLI does.
    pub fn of_file_name(name: &str) -> Option<Self> {
        infer_kind(Path::new(name)).and_then(|kind| Self::try_from(kind).ok())
    }

    /// As the database and the API spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
            Self::Audio => "audio",
        }
    }

    /// The asset kind a file of this kind is imported as.
    pub fn asset_kind(self) -> AssetKind {
        match self {
            Self::Video => AssetKind::Video,
            Self::Image => AssetKind::Image,
            Self::Audio => AssetKind::Audio,
        }
    }

    /// The `Content-Type` a file of this kind with `extension` is served as.
    ///
    /// Only for the extensions [`Kind::of_file_name`] accepts; anything else
    /// is served as bytes, which a browser will not try to run.
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
            _ => Err(format!("{kind:?} is not a library kind")),
        }
    }
}
