//! Which model draws a still, and what each one can and cannot be asked for.
//!
//! **Every limit here is Google's, as documented**, read off
//! <https://ai.google.dev/gemini-api/docs/image-generation> on 2026-10-08
//! (the parity rule, #891: scorsese offers what the provider's API offers, so
//! the page is the checklist). A limit is answered by the model, never matched
//! on wherever a request is built, so a model that arrives or changes has one
//! place to land.
//!
//! # When to use which
//!
//! Speed is not a factor — nobody is in a hurry for a still (the maintainer,
//! 2026-10-07) — so the choice is quality against price per picture, and what
//! a model cannot do decides the rest:
//!
//! - **Nano Banana 2.1**, the default: Google's recommended model for new work,
//!   with better visual quality, text rendering and consistency across
//!   pictures than 2, and *cheaper* per picture at every size it draws ($0.050
//!   for a 2K picture against 2's $0.101). Its input costs three times 2's, a
//!   fraction of a cent for a prompt and a few references.
//! - **Nano Banana 2** (`flash`): the only model that draws 0.5K, the cheapest
//!   test of a prompt at $0.045. Otherwise 2.1 is better and cheaper.
//! - **Lite**: the cheapest picture ($0.034), 1K only, and takes objects only —
//!   no character or style references. Google says it is not tuned for many
//!   references, though it accepts the most objects.
//! - **Pro**: the most expensive ($0.134 at 1K or 2K, $0.24 at 4K), for the most
//!   complex compositions, and the only model that takes style references.

use serde::{Deserialize, Serialize};

use super::{ImageAspect, ImageResolution, ImageThinking, ReferenceKind};

/// Which image model a still is drawn with.
///
/// Per asset, for [`VideoModel`](crate::asset::VideoModel)'s reason: the
/// background a whole film sits on and a throwaway insert are not worth the
/// same money, and only the person editing knows which is which. The module
/// doc has when to pick which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageModel {
    /// Gemini Nano Banana 2.1, and the default: better than 2 and cheaper per
    /// picture at every size it draws, which is 1K, 2K and 4K.
    #[default]
    #[serde(rename = "nano_banana_2.1")]
    NanoBanana21,
    /// Gemini 3.1 Flash Image (Nano Banana 2): every size, 0.5K included.
    Flash,
    /// Gemini 3.1 Flash Lite Image: the cheapest picture, 1K only, objects
    /// only as references.
    Lite,
    /// Gemini 3 Pro Image (Nano Banana Pro): the dearest, for the most complex
    /// compositions, and the only model taking style references.
    Pro,
}

impl ImageModel {
    /// Every model, the default first.
    pub const ALL: [Self; 4] = [Self::NanoBanana21, Self::Flash, Self::Lite, Self::Pro];

    /// The model's name as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NanoBanana21 => "nano_banana_2.1",
            Self::Flash => "flash",
            Self::Lite => "lite",
            Self::Pro => "pro",
        }
    }

    /// True when this model draws at `resolution`.
    ///
    /// 0.5K is Flash's alone; Lite draws 1K only; the rest draw 1K to 4K.
    pub fn supports(self, resolution: ImageResolution) -> bool {
        match self {
            Self::Flash => true,
            Self::Lite => resolution == ImageResolution::K1,
            Self::NanoBanana21 | Self::Pro => resolution != ImageResolution::K05,
        }
    }

    /// True when this model draws `aspect`.
    ///
    /// The four long strips (`1:4`, `4:1`, `1:8`, `8:1`) are 2.1's and
    /// Flash's; every model draws the other ten.
    pub fn draws(self, aspect: ImageAspect) -> bool {
        match self {
            Self::NanoBanana21 | Self::Flash => true,
            Self::Lite | Self::Pro => !aspect.is_strip(),
        }
    }

    /// The most references of `kind` one still from this model may name.
    ///
    /// Google's table, which caps each kind separately within fourteen in
    /// all: 2.1 and Flash take ten objects and four characters; Lite fourteen
    /// objects; Pro six objects, five characters and three styles. Zero means
    /// the model takes none of that kind.
    pub const fn references(self, kind: ReferenceKind) -> usize {
        match (self, kind) {
            (Self::NanoBanana21 | Self::Flash, ReferenceKind::Object) => 10,
            (Self::NanoBanana21 | Self::Flash, ReferenceKind::Character) => 4,
            (Self::Lite, ReferenceKind::Object) => 14,
            (Self::Pro, ReferenceKind::Object) => 6,
            (Self::Pro, ReferenceKind::Character) => 5,
            (Self::Pro, ReferenceKind::Style) => 3,
            (Self::NanoBanana21 | Self::Flash | Self::Lite, _) => 0,
        }
    }

    /// The thinking levels a request to this model may name, in order.
    ///
    /// Empty for Pro: it thinks, as every Gemini 3 image model does, but the
    /// page offers no level to choose.
    pub const fn thinking_levels(self) -> &'static [ImageThinking] {
        match self {
            Self::NanoBanana21 => &[
                ImageThinking::Minimal,
                ImageThinking::Medium,
                ImageThinking::High,
            ],
            Self::Flash | Self::Lite => &[ImageThinking::Minimal, ImageThinking::High],
            Self::Pro => &[],
        }
    }

    /// The level this model thinks at when the request names none — `medium`
    /// on 2.1, `minimal` on Flash and Lite, and none to name on Pro.
    pub const fn default_thinking(self) -> Option<ImageThinking> {
        match self {
            Self::NanoBanana21 => Some(ImageThinking::Medium),
            Self::Flash | Self::Lite => Some(ImageThinking::Minimal),
            Self::Pro => None,
        }
    }

    /// The size a still is drawn at when its request names none.
    ///
    /// 2K wherever it is drawn: a 16:9 2K still is 2752×1536, which covers a
    /// 1080p frame with room left for the slow push or the pan a still is
    /// usually given — at 1K it would be enlarged to fill the frame before
    /// anything moved. 1K on Lite, because it is the only size Lite draws, and
    /// a default the model refuses is not a default.
    pub const fn default_resolution(self) -> ImageResolution {
        match self {
            Self::NanoBanana21 | Self::Flash | Self::Pro => ImageResolution::K2,
            Self::Lite => ImageResolution::K1,
        }
    }
}
