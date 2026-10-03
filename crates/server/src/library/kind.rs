//! What a library file is: the three kinds a file can be imported as, and
//! MIDI.
//!
//! A `.mid` is a library file like any other (#678): uploaded the way a song
//! is, listed and deleted the same way. What sets it apart is that it is not
//! media — no clip can show it — so it has no asset kind and no thumbnail;
//! `synth_import` reads one into a song recipe, and `synth_export` keeps the
//! one it writes here.

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
    /// A Standard MIDI File: notes, not sound. Read into a recipe by
    /// `synth_import`, never placed as a clip.
    Midi,
}

/// The extensions a MIDI file arrives with.
const MIDI: &[&str] = &["mid", "midi"];

impl Kind {
    /// What a file called `name` is, by the list `scorsese import` uses — so
    /// the web app welcomes exactly the media the CLI does — and a `.mid` or
    /// `.midi`, which that list leaves out because it is not media; the CLI
    /// takes one by path in `synth import`.
    pub fn of_file_name(name: &str) -> Option<Self> {
        let path = Path::new(name);
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        if MIDI.contains(&extension.as_str()) {
            return Some(Self::Midi);
        }
        infer_kind(path).and_then(|kind| Self::try_from(kind).ok())
    }

    /// As the database and the API spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Midi => "midi",
        }
    }

    /// What the kind is called in a sentence: "the MIDI file".
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "picture",
            Self::Audio => "sound",
            Self::Midi => "MIDI",
        }
    }

    /// The asset kind a file of this kind is imported as — `None` for MIDI,
    /// which a project takes in as a recipe (`synth_import`), not as a file.
    pub fn asset_kind(self) -> Option<AssetKind> {
        match self {
            Self::Video => Some(AssetKind::Video),
            Self::Image => Some(AssetKind::Image),
            Self::Audio => Some(AssetKind::Audio),
            Self::Midi => None,
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
            (Self::Midi, "mid" | "midi") => "audio/midi",
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
            "midi" => Ok(Self::Midi),
            _ => Err(format!("{kind:?} is not a library kind")),
        }
    }
}
