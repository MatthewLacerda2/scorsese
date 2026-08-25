//! How a clip's picture meets the raster: fitted, cropped, anchored,
//! pivoted.
//!
//! Four fields that answer four parts of one question — *what of the source
//! is shown*, *how it is scaled to the frame*, *where in the frame it
//! sits*, and *what point of itself it turns about*. They are here rather
//! than beside [`Clip`](super::Clip) because a clip is a placement in **time**
//! and these are placements in **space**, and because between them they carry
//! most of the reasoning in this file.
//!
//! All four are absent by default and every default is what the format did
//! before the field existed, so a document that says nothing about any of them
//! means exactly what it always did.

use serde::{Deserialize, Serialize};

/// How a clip's source is fitted into the render's raster.
///
/// The raster is a render setting, and the project is not supposed to care what
/// it is. So this says what the author *meant* — the whole thing with bars
/// allowed, cover it and crop the overflow, or leave it alone — and lets the
/// render work out the pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Scale to fit inside the raster, keeping proportions. What is left over
    /// is **transparent**, so the tracks below show through it.
    #[default]
    Fit,
    /// Scale to cover the raster, keeping proportions, cropping the overflow
    /// off the edges. What a background plate that must not have bars wants.
    Fill,
    /// No scaling at all: the source arrives at its own pixel size, resting
    /// centred, and `transform.position.*` offsets it from there.
    ///
    /// This is how something is placed at a size it was authored at. Scaling a
    /// 64×64 logo to fit makes its on-screen size a function of the render's
    /// resolution, so the factor that shrinks it back means nothing to a reader
    /// and stops meaning it the moment the render changes size.
    Native,
}

impl Fit {
    /// True for [`Fit::Fit`] — what a clip that says nothing means. Keeps the
    /// field out of documents that do not set it.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Fit)
    }
}

/// Which edge of the frame a layer's matching edge is measured from.
///
/// **The format could express where a layer ended up and not what was meant.**
/// A title column beside a picture — the commonest arrangement in the medium —
/// was written as `transform.position.x: -580`, a number derived on paper from
/// the block's width and the fact that text is drawn centred. Nobody can read a
/// layout back out of that; putting the text on the other side is a
/// recomputation rather than one word; and lengthening the title moves it,
/// because a centred block grows both ways.
///
/// Under an anchor the same layout is `left` with an offset of `90` — a margin,
/// which is the thing the author actually had in mind — and swapping sides is
/// `left` → `right` with the number unchanged.
///
/// Absent means centred on both axes, which is what every layer did before the
/// field existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Anchor {
    /// Which vertical edge the horizontal offset is measured from.
    pub x: AnchorX,
    /// Which horizontal edge the vertical offset is measured from.
    pub y: AnchorY,
}

impl Anchor {
    /// True for centred on both axes — what a clip that says nothing means, and
    /// what keeps the field out of documents that do not set it.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// The horizontal edge an offset is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorX {
    /// The layer's left edge, from the frame's left edge.
    Left,
    /// The layer's centre, from the frame's centre.
    #[default]
    Center,
    /// The layer's right edge, from the frame's right edge — so a positive
    /// offset moves it *further in*, and the same number means the same margin
    /// as it does on the left.
    Right,
}

/// The vertical edge an offset is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorY {
    /// The layer's top edge, from the frame's top edge.
    Top,
    /// The layer's centre, from the frame's centre.
    #[default]
    Center,
    /// The layer's bottom edge, from the frame's bottom edge, so a positive
    /// offset moves it further in.
    Bottom,
}

/// The point of a layer's own box that its scale and rotation turn about.
///
/// **Without it, growing a layer from one edge is two coupled keyframe tracks
/// and arithmetic done on paper.** A progress bar that fills from the left is
/// a scale on `x` from `0` to `1`; because scale turns about the centre, the
/// bar also has to be slid left by `(s − 1) / 2` on every frame, which is a
/// second track holding a number nobody can read back as *the left edge stays
/// put*. Worse, the two only agree while the scale is linear in time: put an
/// `ease_out` on it and the bar slides while it grows. The document still
/// validates and the render still succeeds — the only symptom is watching it.
///
/// With an origin the same bar is one track and one word, and the easing is
/// free.
///
/// **Scale and rotation both**, because a card hinging on its left edge is the
/// same request as a bar filling from it, and one pivot for the two is the
/// coherent reading of *the point the transform turns about*. `position` is
/// applied after both and is unaffected — a pivot cannot move a layer that is
/// not being scaled or turned, which is what makes the field free to set.
///
/// **The layer's own box**, which is the raster its pixels arrive on: for a
/// decoded picture that rectangle is the picture, and for anything drawn — a
/// title, a shape, an icon — it is the render's raster, since those are drawn
/// at full size with the content placed inside them by [`Anchor`].
///
/// **Not to be confused with [`Anchor`]**, which answers a different question:
/// an anchor says which edge of the *frame* a layer rests against, an origin
/// says which point of the *layer* its own transform pivots on. An anchor
/// decides where a layer sits; an origin decides what it does about itself
/// once it is there.
///
/// Absent means centred on both axes, which is what every layer did before the
/// field existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Origin {
    /// Which point across the layer's box the transform pivots on.
    pub x: OriginX,
    /// Which point down the layer's box the transform pivots on.
    pub y: OriginY,
}

impl Origin {
    /// True for the layer's own centre — what a clip that says nothing means,
    /// and what keeps the field out of documents that do not set it.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The pivot as fractions of the layer's own box: `(0.0, 0.0)` is its
    /// top-left corner and `(1.0, 1.0)` its bottom-right.
    ///
    /// Fractions rather than pixels, so the answer is the same whatever
    /// raster the layer turns out to arrive at — and one function rather than
    /// a match at each use, so the compositor and anything asking where a
    /// layer landed cannot disagree about what `left` means.
    pub fn fractions(self) -> (f64, f64) {
        let across = match self.x {
            OriginX::Left => 0.0,
            OriginX::Center => 0.5,
            OriginX::Right => 1.0,
        };
        let down = match self.y {
            OriginY::Top => 0.0,
            OriginY::Center => 0.5,
            OriginY::Bottom => 1.0,
        };
        (across, down)
    }
}

/// Which point across a layer's own box its transform pivots on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginX {
    /// The layer's left edge, which stays put as the layer grows rightward.
    Left,
    /// The layer's middle — what scaling and turning always did.
    #[default]
    Center,
    /// The layer's right edge, which stays put as the layer grows leftward.
    Right,
}

/// Which point down a layer's own box its transform pivots on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginY {
    /// The layer's top edge, which stays put as the layer grows downward.
    Top,
    /// The layer's middle — what scaling and turning always did.
    #[default]
    Center,
    /// The layer's bottom edge, which stays put as the layer grows upward.
    Bottom,
}

/// A rectangle of the source, in fractions of it.
///
/// **The asset is never touched.** Cropping by cutting the file down is the one
/// place the format's premise — a document describing an edit over unmodified
/// assets — currently has to be broken to do ordinary work, and a project that
/// does it stops being a description of an edit and becomes a description of a
/// result: the original pixels are gone, nothing records that a sidebar was
/// removed or from where, and the recorded `sha256` describes a file no camera
/// and no capture ever produced.
///
/// **Fractions of the source, not source pixels**, and the reasoning matters
/// more than the choice. A fraction survives the asset being *replaced* by a
/// higher-resolution capture of the same thing — re-shoot the screenshot at 4K
/// and the crop still means the same region, where in pixels it would silently
/// mean a different one. That is exactly the change-your-mind-later case this
/// exists for, so the unit must not be the one that breaks it. A fraction also
/// validates from the document alone, where a pixel rectangle would need the
/// source's dimensions, which are only recorded if something probed the asset.
///
/// This is a different question from `transform.position`, which is a fraction
/// of the **output** raster. A crop is against the **source** raster, and the
/// two do not have to answer the same way.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crop {
    /// Left edge, as a fraction of the source's width.
    pub x: f64,
    /// Top edge, as a fraction of the source's height.
    pub y: f64,
    /// How much of the source's width is kept.
    pub width: f64,
    /// How much of the source's height is kept.
    pub height: f64,
}

impl Default for Crop {
    /// The whole source — what an absent `crop` means.
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl Crop {
    /// Each edge, paired with the name `project.json` spells it — so a message
    /// about one names the field the author wrote rather than a description of
    /// it.
    pub fn edges(&self) -> [(&'static str, f64); 4] {
        [
            ("x", self.x),
            ("y", self.y),
            ("width", self.width),
            ("height", self.height),
        ]
    }

    /// True when the rectangle is inside the source and encloses some of it.
    ///
    /// Checkable from the document alone, which is the point of fractions: a
    /// pixel rectangle would need the source's dimensions, and those are only
    /// recorded if something probed the asset.
    pub fn is_within_source(&self) -> bool {
        self.edges().iter().all(|&(_, value)| value.is_finite())
            && self.width > 0.0
            && self.height > 0.0
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.width <= 1.0
            && self.y + self.height <= 1.0
    }
}
