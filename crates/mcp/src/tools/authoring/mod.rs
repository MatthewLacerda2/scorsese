//! Writing down the assets nothing brings in, and the lanes they sit on.
//!
//! Four asset kinds have no file behind them — a caption, a colour card, a box,
//! a symbol — and until these verbs existed the only way to add one was to send
//! the whole `project.json` back. On a captioned cut that is tens of kilobytes
//! per line of text, which is how a real session came to bypass the guarded
//! path altogether and overwrite work `synth_bake` had just written.
//!
//! **One verb per kind, and that is the self-describing rule deciding it.**
//! What a kind *requires* differs by kind: a colour asset must have a colour, a
//! symbol must have a name and a size. A single `asset_new` taking a `kind` and
//! a free-form block could not say any of that in its schema — the block would
//! be one undescribed object, which is exactly the thing `tests/described.rs`
//! exists to refuse. `synth_new` is the same shape one kind further along.
//!
//! **`asset_set` is one verb, for the mirror-image reason.** It requires
//! nothing but the asset, so every field on it is optional by construction and
//! each one still describes itself and says which kinds it belongs to. Four
//! set-verbs would be four schemas restating the same eight adjectives.

mod color;
mod fill;
mod icon;
mod remove;
mod set;
mod shape;
mod text;
mod track;

pub(crate) use color::ColorNew;
pub(crate) use icon::IconNew;
pub(crate) use remove::{AssetRemove, TrackRemove};
pub(crate) use set::AssetSet;
pub(crate) use shape::ShapeNew;
pub(crate) use text::TextNew;
pub(crate) use track::TrackNew;

use schemars::JsonSchema;
use scorsese_core::{AuthorError, Project, Rgba, TextAlign};
use serde::Deserialize;

use crate::tools::args;

/// An optional text argument, absent when it is blank — which is also how
/// the id a caller asked the new thing to be called is read.
fn maybe(given: Option<&str>) -> Option<String> {
    args::given(given).map(str::to_owned)
}

/// A whole number in OpenType's `wght` range — anything else is not a weight.
fn weight(given: Option<u32>) -> Result<Option<u16>, String> {
    given
        .map(|number| {
            u16::try_from(number)
                .map_err(|_| "`weight` has to be a whole number, 1 to 1000".to_owned())
        })
        .transpose()
}

/// A colour, read the way the document writes one; blank is not given.
fn color(given: Option<&str>, key: &str) -> Result<Option<Rgba>, String> {
    let Some(text) = args::given(given) else {
        return Ok(None);
    };
    text.parse()
        .map(Some)
        .map_err(|problem| format!("`{key}`: {problem}"))
}

/// Which edge the lines line up against.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Align {
    Left,
    Center,
    Right,
}

impl From<Align> for TextAlign {
    fn from(align: Align) -> Self {
        match align {
            Align::Left => Self::Left,
            Align::Center => Self::Center,
            Align::Right => Self::Right,
        }
    }
}

/// How a refusal from the model reads on the wire: the reason, and the promise
/// that goes with every one of them.
fn refused(error: AuthorError) -> String {
    format!("{error} — nothing was written")
}

/// Writes the document back, or says why it could not.
fn save(project: &Project, dir: &std::path::Path) -> Result<(), String> {
    project
        .save(dir)
        .map_err(|error| format!("saving the project: {error}"))
}

// The appearance fields, described once wherever they appear.
//
// A `size` means the same thing on a caption and on a symbol, and a `color`
// is written the same way on all four kinds — so two descriptions of either
// would be two chances to drift. Each tool's arguments take the ones its kind
// has, as `#[schemars(description = …)]`.

/// `font`.
const FONT: &str = "The face: a name this build ships — `sans`, `serif` — or a \
                    path to a font file inside the project.";
/// `weight`.
const WEIGHT: &str = "How heavy the glyphs are, 1 to 1000 on the usual scale where \
                      400 is regular and 700 bold. Read from a variable font only.";
/// `italic`.
const ITALIC: &str = "Set it in the family's italic — a different drawing, not the \
                      upright leaned over. Only for a face this build ships.";
/// `size`.
const SIZE: &str = "How big, as a fraction of the frame's HEIGHT: 0.1 is a tenth \
                    of the picture. Means the same at every render resolution.";
/// `color`, where it is one colour.
const COLOR: &str = "The colour, as `#rrggbb` — or `#rrggbbaa` for one you can see \
                     through, which composites over whatever is under it.";
/// `align`.
const ALIGN: &str = "Which edge the lines line up against inside the wrapped \
                     block. Default `center`, which is what a title wants.";
/// `line_height`.
const LINE_HEIGHT: &str = "Baseline to baseline, as a multiple of `size`. 1.0 sets the \
                           lines solid; the default 1.25 leaves a readable gap.";
/// `max_width`.
const MAX_WIDTH: &str = "How wide the text runs before it wraps, as a fraction of the \
                         frame's WIDTH. Default 0.9, a margin down each side.";
/// What `fill` is for, ahead of how a gradient is written.
const FILL: &str = "What the inside of the shape is painted, as `#rrggbb` (or `#rrggbbaa`). \
                    Leave it out for a see-through middle — a callout over footage. The \
                    border stays one colour.";
/// `stroke`.
const STROKE: &str = "The rim, as `#rrggbb`. On a shape it is the border; on a \
                      caption it is an outline added OUTSIDE the letterform, \
                      which is what keeps burned-in words legible over footage. \
                      Left out, there is no rim at all.";
/// `stroke_width`.
const STROKE_WIDTH: &str = "How thick that rim is. On a shape or a caption, a fraction \
                            of the frame's height (0.004 and 0.002 by default); on an \
                            icon, a fraction of the icon's own box, so it scales with \
                            the symbol.";
/// `width`.
const WIDTH: &str = "Across, as a fraction of the frame's width. A closed shape \
                     only — an arrow is its two endpoints.";
/// `height`.
const HEIGHT: &str = "Down, as a fraction of the frame's height. A closed shape \
                      only.";
/// `radius`.
const RADIUS: &str = "How rounded a rectangle's corners are, as a fraction of its \
                      own shorter side: 0 is square and 0.5 a pill.";

/// The `id to call it` argument, worded once for the four kinds that share it.
fn id_described(what: &str) -> String {
    format!(
        "What to call the new asset. Optional: without it an id is derived from {what} \
         and suffixed until it is free, and the reply says which one it wrote. An id \
         already in use is refused rather than quietly changed, because you are about \
         to write it onto a clip."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A colour that does not parse is named; a blank one is not given.
    #[test]
    fn a_colour_is_read_or_named() {
        assert_eq!(color(Some("  "), "stroke"), Ok(None));
        let refused = color(Some("pink"), "stroke").expect_err("not a colour");
        assert!(refused.starts_with("`stroke`:"), "{refused}");
    }

    /// A weight past what OpenType can say is refused rather than wrapped.
    #[test]
    fn a_weight_is_a_u16_or_refused() {
        assert_eq!(weight(Some(700)), Ok(Some(700)));
        assert!(weight(Some(70_000)).is_err());
    }
}
