//! What a generated still asks for, beyond the sentence.
//!
//! The sibling of [`super::video`], and shorter, because a still has no length.
//! What each model can be asked for differs — sizes, shapes, how many
//! references of each kind, which thinking levels — and every one of those is
//! asked of [`ImageModel`] rather than matched on wherever a request is built.
//! [`model`]'s doc has when to pick which model.
//!
//! **Resolution is the money lever**, so it is a field and never implied. The
//! output is billed per picture at a price fixed by its model and size. Aspect
//! is a field of its own beside it rather than a consequence of it: a model
//! draws every aspect it offers at every size it offers, so the two are
//! independent choices and one field could not hold both.

mod model;
mod shape;

use serde::{Deserialize, Serialize};

use super::AssetId;

pub use model::ImageModel;
pub use shape::{ImageAspect, ImageResolution};

/// What a reference picture is there to keep, which is what Google caps.
///
/// The vendor's API is never told: every reference is handed over as a
/// picture, in order, and the prompt says what each is for. The kind is in the
/// brief because the limits are per kind ([`ImageModel::references`]), and a
/// brief over one is refused rather than sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// A thing to include faithfully: a product, a logo, a prop, a place.
    Object,
    /// A person or creature to keep looking like themselves.
    Character,
    /// A look to draw in: a palette, a medium, a period.
    Style,
}

impl ReferenceKind {
    /// Every kind, in the order a request hands them over.
    pub const ALL: [Self; 3] = [Self::Object, Self::Character, Self::Style];

    /// The field of [`ImageRequest`] that holds this kind, as `project.json`
    /// spells it.
    pub const fn field(self) -> &'static str {
        match self {
            Self::Object => "reference_images",
            Self::Character => "character_images",
            Self::Style => "style_images",
        }
    }
}

/// How hard a model thinks before it draws.
///
/// Google's own control: more thinking for a complex composition, less for a
/// plain one. It changes the picture and the thinking tokens billed, so it is
/// part of the brief. Which levels a model takes, and its default, are
/// [`ImageModel::thinking_levels`] and [`ImageModel::default_thinking`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageThinking {
    /// The least: Flash's and Lite's default.
    Minimal,
    /// 2.1's default. 2.1 only.
    Medium,
    /// The most, for a crowded or exacting composition.
    High,
}

impl ImageThinking {
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
    /// Which model draws it.
    pub model: ImageModel,
    /// How large it comes back. Absent means the model's own default — see
    /// [`ImageModel::default_resolution`] and [`ImageRequest::size`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ImageResolution>,
    /// Which shape it is.
    pub aspect: ImageAspect,
    /// How hard the model thinks first. Absent means the model's own default
    /// — see [`ImageRequest::thinking`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ImageThinking>,
    /// Pictures of **objects** to include faithfully, named by asset id — at
    /// most [`ImageModel::references`] of them for [`ReferenceKind::Object`].
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
    /// Pictures of a **style** to draw in. Pro only.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub style_images: Vec<AssetId>,
}

impl ImageRequest {
    /// The size this request is drawn at, its model's default included.
    pub fn size(&self) -> ImageResolution {
        self.resolution.unwrap_or(self.model.default_resolution())
    }

    /// The thinking level this request is drawn at, its model's default
    /// included — `None` only on a model with no level to name.
    pub fn thinking(&self) -> Option<ImageThinking> {
        self.thinking.or(self.model.default_thinking())
    }

    /// The references of one kind.
    pub fn references_of(&self, kind: ReferenceKind) -> &[AssetId] {
        match kind {
            ReferenceKind::Object => &self.reference_images,
            ReferenceKind::Character => &self.character_images,
            ReferenceKind::Style => &self.style_images,
        }
    }

    /// The references of one kind, to edit.
    pub fn references_of_mut(&mut self, kind: ReferenceKind) -> &mut Vec<AssetId> {
        match kind {
            ReferenceKind::Object => &mut self.reference_images,
            ReferenceKind::Character => &mut self.character_images,
            ReferenceKind::Style => &mut self.style_images,
        }
    }

    /// Every reference, in the order they are handed over: objects, then
    /// characters, then styles.
    pub fn references(&self) -> impl Iterator<Item = &AssetId> {
        ReferenceKind::ALL
            .into_iter()
            .flat_map(|kind| self.references_of(kind))
    }
}
