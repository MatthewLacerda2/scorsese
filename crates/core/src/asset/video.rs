//! What a generated video asks for, beyond the sentence.
//!
//! A prompt says what the shot is *of*. Everything here says what the shot
//! *is* — how long, how big, which way up, which stills it is built from — and
//! every one of those changes the video that comes back. So they live in the
//! document beside the prompt, they are hashed into the brief with it, and
//! changing any of them makes the asset stale exactly as rewording the
//! sentence does.
//!
//! The provider's constraints are the interesting part. Several of these
//! fields are not independent: choosing 1080p fixes the length, and choosing
//! the cheaper tier removes reference images altogether. Those couplings are
//! answered here, by asking the value what it supports, rather than matched on
//! wherever a request happens to be built — so the next capability that
//! differs between tiers has one place to land.

use serde::{Deserialize, Serialize};

use super::AssetId;

/// The most reference images one request may carry.
pub const MAX_REFERENCE_IMAGES: usize = 3;

/// Which tier of the video model a shot is generated with.
///
/// A per-shot judgement rather than a project-wide setting: the shot that
/// carries the film and the cutaway behind the narration are not worth the
/// same money, and nothing but the person editing can say which is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoModel {
    /// The default tier: the one to reach for unless there is a reason not to.
    #[default]
    Fast,
    /// Cheaper, and less capable — see [`VideoModel::supports_reference_images`].
    /// For shots where the picture matters less than the money.
    Lite,
}

impl VideoModel {
    /// True when this tier accepts [`VideoRequest::reference_images`].
    ///
    /// The one capability that differs between the tiers today, and the reason
    /// this is a question rather than a comparison: switching a shot to the
    /// cheaper tier to save money must not silently discard the images that
    /// were keeping a character looking like themselves.
    pub fn supports_reference_images(self) -> bool {
        matches!(self, Self::Fast)
    }

    /// The tier's name as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Lite => "lite",
        }
    }
}

/// How large the generated video comes back.
///
/// Not a free number: the provider offers two, and one of them fixes the
/// length — see [`VideoRequest::eight_second_lock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VideoResolution {
    /// 1280×720. The cheaper raster, and the only one that leaves the length
    /// free to choose.
    #[serde(rename = "720p")]
    P720,
    /// 1920×1080, and the default — a generated shot sits in a cut beside
    /// imported footage, and being the smaller thing in it is a choice nobody
    /// should make by accident.
    #[default]
    #[serde(rename = "1080p")]
    P1080,
}

impl VideoResolution {
    /// True when this raster is only generated at eight seconds.
    pub fn locks_length(self) -> bool {
        matches!(self, Self::P1080)
    }

    /// The raster's name as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
        }
    }
}

/// How long a generated shot is.
///
/// Three lengths, not a number: the provider generates in fixed sizes and
/// refuses anything else. Longer shots come from extending a generated clip,
/// which is a different request and is not modelled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub enum ClipSeconds {
    /// Four seconds. Only at 720p, and only without images.
    Four,
    /// Six seconds. Only at 720p, and only without images.
    Six,
    /// Eight seconds, and the default — every other choice is a request to
    /// generate *less*, and the constraints mean it is often not available.
    #[default]
    Eight,
}

impl ClipSeconds {
    /// Every length the provider will generate, shortest first.
    pub const ALL: [Self; 3] = [Self::Four, Self::Six, Self::Eight];

    /// The length as a number of seconds.
    pub const fn get(self) -> u32 {
        match self {
            Self::Four => 4,
            Self::Six => 6,
            Self::Eight => 8,
        }
    }
}

impl From<ClipSeconds> for u32 {
    fn from(seconds: ClipSeconds) -> Self {
        seconds.get()
    }
}

impl TryFrom<u32> for ClipSeconds {
    type Error = String;

    fn try_from(seconds: u32) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.get() == seconds)
            .ok_or_else(|| format!("a generated shot is 4, 6 or 8 seconds long, never {seconds}"))
    }
}

/// Which way up a generated shot is.
///
/// Its own field rather than something read off the render settings: the
/// aspect is baked into the file the provider returns, so it is a fact about
/// the asset and outlives any one render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Aspect {
    /// Landscape, and the default.
    #[default]
    #[serde(rename = "16:9")]
    Wide,
    /// Portrait — for a cut that is going somewhere it will be watched upright.
    #[serde(rename = "9:16")]
    Tall,
}

impl Aspect {
    /// The aspect as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wide => "16:9",
            Self::Tall => "9:16",
        }
    }
}

/// Why a request's length is fixed at eight seconds.
///
/// Carried rather than inferred, because "you cannot have six seconds" is a
/// third of an explanation: what a person needs is which of their other
/// choices took the option away, since that is the one they might reconsider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthLock {
    /// The 1080p raster is only generated at eight seconds.
    Resolution,
    /// A request carrying reference images is only generated at eight seconds.
    ReferenceImages,
    /// A request interpolating between two stills is only generated at eight
    /// seconds.
    Interpolation,
}

impl LengthLock {
    /// The choice that fixed the length, named the way a message should say it.
    pub const fn cause(self) -> &'static str {
        match self {
            Self::Resolution => "1080p",
            Self::ReferenceImages => "reference images",
            Self::Interpolation => "a first and last image",
        }
    }
}

/// Everything a `generated_video` asset asks for beyond its prompt.
///
/// Every field has a default, so an asset that says nothing here is still a
/// complete request — the shortest useful brief is a sentence and nothing
/// else. Absent from the document entirely means the same thing as present and
/// empty, which is why [`crate::Asset::video_request`] exists rather than
/// callers reading the option.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VideoRequest {
    /// Which tier to generate with.
    pub model: VideoModel,
    /// How large the result comes back.
    pub resolution: VideoResolution,
    /// How long the result is.
    pub seconds: ClipSeconds,
    /// Which way up the result is.
    pub aspect: Aspect,
    /// A still to open on, named by asset id.
    ///
    /// By id and never by path, exactly as a clip names its asset: a path here
    /// would be a second way to point at media, and an absolute one would break
    /// the promise that a project survives being copied to another machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_image: Option<AssetId>,
    /// A still to end on. Only meaningful beside [`VideoRequest::first_image`],
    /// because what it asks for is the journey between the two.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_image: Option<AssetId>,
    /// Stills of a subject to keep looking like itself across shots, at most
    /// [`MAX_REFERENCE_IMAGES`] of them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reference_images: Vec<AssetId>,
}

impl VideoRequest {
    /// True when this request asks for the journey between two stills.
    pub fn interpolates(&self) -> bool {
        self.first_image.is_some() && self.last_image.is_some()
    }

    /// Why this request's length is fixed, or `None` when it is free to choose.
    ///
    /// Resolution is reported first when several apply, because it is the one
    /// most likely to be the choice someone would trade away — dropping to
    /// 720p is a smaller loss than giving up the images the shot is built from.
    pub fn eight_second_lock(&self) -> Option<LengthLock> {
        if self.resolution.locks_length() {
            Some(LengthLock::Resolution)
        } else if !self.reference_images.is_empty() {
            Some(LengthLock::ReferenceImages)
        } else if self.interpolates() {
            Some(LengthLock::Interpolation)
        } else {
            None
        }
    }

    /// Every asset this request names a still from, in document order.
    ///
    /// One iterator rather than three fields, because everything asked of an
    /// image here is asked of all of them: that it exists, and that it is a
    /// picture.
    pub fn images(&self) -> impl Iterator<Item = &AssetId> {
        self.first_image
            .iter()
            .chain(self.last_image.iter())
            .chain(self.reference_images.iter())
    }
}
