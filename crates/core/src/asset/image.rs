//! What a generated still asks for, beyond the sentence.
//!
//! The sibling of [`super::video`], and shorter, because a still has no length
//! and the provider couples fewer of its choices. What it does couple is the
//! one worth stating: the cheaper model only draws at one size. That is asked
//! of the model here — [`ImageModel::supports`] — rather than matched on
//! wherever a request is built, so the next capability that differs between the
//! two has one place to land.
//!
//! **Resolution is the money lever**, so it is a field and never implied. The
//! output is billed per picture at a price fixed by its size alone, and the
//! largest costs three times the smallest. Aspect is a field of its own beside
//! it rather than a consequence of it: the vendor draws every aspect at every
//! size, so the two are independent choices and one field could not hold both.

use serde::{Deserialize, Serialize};

use super::AssetId;

/// The most reference images one still may be drawn from.
///
/// Fourteen, which is what both models accept in total — the larger model
/// splits it into ten pictures of objects and four of characters, a
/// distinction the request does not carry because nothing in the API is told
/// which is which.
pub const MAX_IMAGE_REFERENCES: usize = 14;

/// Which image model a still is drawn with.
///
/// Per asset, for [`VideoModel`](super::VideoModel)'s reason: the background
/// a whole film sits on and a throwaway insert are not worth the same money,
/// and only the person editing knows which is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageModel {
    /// Gemini 3.1 Flash Image, and the default: every size, and the model the
    /// vendor builds for several reference images — which is the whole answer
    /// to a character staying themselves across a hundred generations.
    #[default]
    Flash,
    /// Gemini 3.1 Flash Lite Image: half the price, one size (1K), and not
    /// tuned for many references. For a still where the money matters more
    /// than the picture.
    Lite,
}

impl ImageModel {
    /// The model's name as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flash => "flash",
            Self::Lite => "lite",
        }
    }

    /// True when this model draws at `resolution`.
    pub fn supports(self, resolution: ImageResolution) -> bool {
        match self {
            Self::Flash => true,
            Self::Lite => resolution == ImageResolution::K1,
        }
    }

    /// The size a still is drawn at when its request names none.
    ///
    /// 2K on the full model: a 16:9 2K still is 2752×1536, which covers a
    /// 1080p frame with room left for the slow push or the pan a still is
    /// usually given — at 1K it would be enlarged to fill the frame before
    /// anything moved. 1K on Lite, because it is the only size Lite draws, and
    /// a default the model refuses is not a default.
    pub const fn default_resolution(self) -> ImageResolution {
        match self {
            Self::Flash => ImageResolution::K2,
            Self::Lite => ImageResolution::K1,
        }
    }
}

/// How large a generated still comes back, named the way the vendor prices it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResolution {
    /// 512 pixels on the long side, roughly. The cheapest, for a thumbnail or
    /// a test of a prompt. Full model only.
    #[serde(rename = "0.5K")]
    K05,
    /// About 1024 on the long side. The only size Lite draws.
    #[serde(rename = "1K")]
    K1,
    /// About 2048 on the long side — the full model's default.
    #[serde(rename = "2K")]
    K2,
    /// About 4096 on the long side. For a still that is pushed into a long way.
    #[serde(rename = "4K")]
    K4,
}

impl ImageResolution {
    /// Every size, smallest first.
    pub const ALL: [Self; 4] = [Self::K05, Self::K1, Self::K2, Self::K4];

    /// How it is written in `project.json` and in a price table.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::K05 => "0.5K",
            Self::K1 => "1K",
            Self::K2 => "2K",
            Self::K4 => "4K",
        }
    }
}

/// Which shape a generated still is.
///
/// Every ratio both models draw. Baked into the file the vendor returns, so it
/// is a fact about the asset — and a still that is not the render's shape is
/// not wrong, only fitted, the way any imported photograph is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageAspect {
    /// Landscape, and the default — the shape most cuts are.
    #[default]
    #[serde(rename = "16:9")]
    Wide,
    /// Portrait, for a cut watched upright.
    #[serde(rename = "9:16")]
    Tall,
    /// Square.
    #[serde(rename = "1:1")]
    Square,
    /// The older television shape, landscape.
    #[serde(rename = "4:3")]
    FourThree,
    /// The older television shape, portrait.
    #[serde(rename = "3:4")]
    ThreeFour,
    /// A photograph, landscape.
    #[serde(rename = "3:2")]
    ThreeTwo,
    /// A photograph, portrait.
    #[serde(rename = "2:3")]
    TwoThree,
    /// A print, landscape.
    #[serde(rename = "5:4")]
    FiveFour,
    /// A print, portrait — the shape of most social feeds.
    #[serde(rename = "4:5")]
    FourFive,
    /// Cinema-wide, for a backdrop panned across.
    #[serde(rename = "21:9")]
    Cinema,
}

impl ImageAspect {
    /// Every ratio, in the order a picker should offer them.
    pub const ALL: [Self; 10] = [
        Self::Wide,
        Self::Tall,
        Self::Square,
        Self::FourThree,
        Self::ThreeFour,
        Self::ThreeTwo,
        Self::TwoThree,
        Self::FiveFour,
        Self::FourFive,
        Self::Cinema,
    ];

    /// The ratio as `project.json` and the vendor both spell it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wide => "16:9",
            Self::Tall => "9:16",
            Self::Square => "1:1",
            Self::FourThree => "4:3",
            Self::ThreeFour => "3:4",
            Self::ThreeTwo => "3:2",
            Self::TwoThree => "2:3",
            Self::FiveFour => "5:4",
            Self::FourFive => "4:5",
            Self::Cinema => "21:9",
        }
    }
}

/// Everything a `generated_image` asset asks for beyond its prompt.
///
/// Every field has a default, so a sentence and nothing else is a complete
/// request; absent from the document means every default, which is why
/// [`crate::Asset::image_request`] exists rather than callers reading the
/// option.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ImageRequest {
    /// Which model draws it.
    pub model: ImageModel,
    /// How large it comes back. Absent means the model's own default — see
    /// [`ImageModel::default_resolution`] and [`ImageRequest::size`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ImageResolution>,
    /// Which shape it is.
    pub aspect: ImageAspect,
    /// Pictures of a subject to keep looking like itself, at most
    /// [`MAX_IMAGE_REFERENCES`] of them, named by asset id.
    ///
    /// By id and never by path, as a clip names its asset. A still or another
    /// generated still may be named — one canonical character sheet, generated
    /// once and named by every picture after it, is what this field is for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reference_images: Vec<AssetId>,
}

impl ImageRequest {
    /// The size this request is drawn at, its model's default included.
    pub fn size(&self) -> ImageResolution {
        self.resolution.unwrap_or(self.model.default_resolution())
    }
}
