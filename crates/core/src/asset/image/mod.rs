//! What a generated still asks for, beyond the sentence.
//!
//! The sibling of [`super::video`], and shorter, because a still has no length
//! and no model to choose: every still is Nano Banana 2.1's, and [`limits`]
//! holds what it can be asked for.
//!
//! **Resolution is the money lever**, so it is a field and never implied. The
//! output is billed per picture at a price fixed by its size. Aspect is a
//! field of its own beside it rather than a consequence of it: every aspect is
//! drawn at every size, so the two are independent choices and one field could
//! not hold both.

mod limits;
mod shape;

use serde::{Deserialize, Serialize};

use super::AssetId;

pub use limits::{DEFAULT_IMAGE_RESOLUTION, DEFAULT_IMAGE_THINKING};
pub use shape::{ImageAspect, ImageResolution};

/// What a reference picture is there to keep, which is what Google caps.
///
/// The vendor's API is never told: every reference is handed over as a
/// picture, in order, and the prompt says what each is for. The kind is in the
/// brief because the limits are per kind ([`ReferenceKind::max`]), and a
/// brief over one is refused rather than sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// A thing to include faithfully: a product, a logo, a prop, a place.
    Object,
    /// A person or creature to keep looking like themselves.
    Character,
}

impl ReferenceKind {
    /// Every kind, in the order a request hands them over.
    pub const ALL: [Self; 2] = [Self::Object, Self::Character];

    /// The field of [`ImageRequest`] that holds this kind, as `project.json`
    /// spells it.
    pub const fn field(self) -> &'static str {
        match self {
            Self::Object => "reference_images",
            Self::Character => "character_images",
        }
    }
}

/// How hard the model thinks before it draws.
///
/// Google's own control: more thinking for a complex composition, less for a
/// plain one. It changes the picture and the thinking tokens billed, so it is
/// part of the brief. Absent means [`DEFAULT_IMAGE_THINKING`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageThinking {
    /// The least, for a plain composition.
    Minimal,
    /// The default.
    Medium,
    /// The most, for a crowded or exacting composition.
    High,
}

impl ImageThinking {
    /// Every level, least first.
    pub const ALL: [Self; 3] = [Self::Minimal, Self::Medium, Self::High];

    /// How `project.json` and the vendor both spell it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Medium => "medium",
            Self::High => "high",
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
    /// How large it comes back. Absent means [`DEFAULT_IMAGE_RESOLUTION`] — see
    /// [`ImageRequest::size`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ImageResolution>,
    /// Which shape it is.
    pub aspect: ImageAspect,
    /// How hard the model thinks first. Absent means [`DEFAULT_IMAGE_THINKING`] —
    /// see [`ImageRequest::thinking`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ImageThinking>,
    /// Pictures of **objects** to include faithfully, named by asset id — at
    /// most [`ReferenceKind::max`] of them for [`ReferenceKind::Object`].
    ///
    /// By id and never by path, as a clip names its asset. A still or another
    /// generated still may be named.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reference_images: Vec<AssetId>,
    /// Pictures of **characters** to keep looking like themselves — one
    /// canonical character sheet, generated once and named by every picture
    /// after it, is what this field is for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub character_images: Vec<AssetId>,
}

impl ImageRequest {
    /// The size this request is drawn at, the default included.
    pub fn size(&self) -> ImageResolution {
        self.resolution.unwrap_or(DEFAULT_IMAGE_RESOLUTION)
    }

    /// The thinking level this request is drawn at, the default included.
    pub fn thinking(&self) -> ImageThinking {
        self.thinking.unwrap_or(DEFAULT_IMAGE_THINKING)
    }

    /// The references of one kind.
    pub fn references_of(&self, kind: ReferenceKind) -> &[AssetId] {
        match kind {
            ReferenceKind::Object => &self.reference_images,
            ReferenceKind::Character => &self.character_images,
        }
    }

    /// The references of one kind, to edit.
    pub fn references_of_mut(&mut self, kind: ReferenceKind) -> &mut Vec<AssetId> {
        match kind {
            ReferenceKind::Object => &mut self.reference_images,
            ReferenceKind::Character => &mut self.character_images,
        }
    }

    /// Every reference, in the order they are handed over: objects, then
    /// characters.
    pub fn references(&self) -> impl Iterator<Item = &AssetId> {
        ReferenceKind::ALL
            .into_iter()
            .flat_map(|kind| self.references_of(kind))
    }
}
