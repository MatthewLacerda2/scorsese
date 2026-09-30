//! Turning a text asset into a layer's worth of pixels.
//!
//! Two things happen here that cannot happen in `scorsese-compositor`, and
//! nothing else does. **Fonts come off disk**: a project may name a font file
//! of its own, and opening files is this crate's side of the boundary — the
//! compositor takes bytes. **Fractions become pixels**: a project stores a size
//! as a fraction of the frame so that one document reads the same at 720p and
//! at 4K, and the raster it is a fraction *of* is a render setting, known here.
//!
//! The drawing itself is entirely the compositor's, and what comes out is an
//! ordinary layer. There is no text path through the renderer beyond this
//! module: a title is composited, transformed and faded by exactly the code a
//! video clip goes through. What [`typing`] adds is the one way a text layer
//! differs — a reveal or a count makes its pixels change from frame to frame,
//! so it is drawn again for each one rather than once for the clip.

mod check;
mod typing;

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use scorsese_compositor::text::{self, Edge, Figures, Font, Slant, Style};
use scorsese_compositor::{Area, Resolution};
use scorsese_core::{Anchor, Asset, Clip, FontChoice, TextStyle};

use crate::error::RenderError;

pub use check::{UncoveredGlyphs, UnknownFont, uncovered_glyphs, unknown_fonts};
pub(crate) use typing::Typing;

/// Parses one face, saying which asset asked for it when it will not parse.
///
/// The error names a path even for a shipped face, where there is none to name:
/// the face's own name goes in its place, because "`sans` does not reach weight
/// 100" is the useful sentence and a blank path is not.
fn open(key: &Face, asset: &Asset) -> Result<Font, RenderError> {
    let unusable = |path: PathBuf, detail: String| RenderError::UnusableFont {
        asset: asset.id.to_string(),
        path,
        detail,
    };
    match key {
        Face::Shipped(name, weight, slant) => Font::shipped(name, Some(*weight), *slant)
            .map_err(|error| unusable(PathBuf::from(name), error.to_string())),
        Face::File(path, weight) => {
            let bytes =
                std::fs::read(path).map_err(|source| unusable(path.clone(), source.to_string()))?;
            Font::from_bytes(&bytes, *weight)
                .map_err(|error| unusable(path.clone(), error.to_string()))
        }
    }
}

/// Draws text assets, holding on to the fonts it has opened.
///
/// Kept for a whole render rather than made per clip: parsing a face costs
/// milliseconds, and a cut with a title on every shot would otherwise pay that
/// for each one.
///
/// **Keyed by face *and* weight**, because one variable file is many faces: a
/// project setting its titles in Inter at 800 and its captions in the same face
/// at 400 has two instances, and they must not share a cache slot. That is the
/// whole point of the feature — one file where the per-weight file tax used to
/// be — so the cache has to be able to hold both at once.
///
/// A shipped face at its default weight is the exception and is not in here at
/// all: it is a `&'static` the compositor parses once per process, which is
/// what every project written before `weight` existed asks for.
#[derive(Debug, Default)]
pub(crate) struct Painter {
    /// Shared rather than owned, because a text layer that reveals or counts
    /// takes its face with it into the workers that draw its frames.
    fonts: HashMap<Face, Arc<Font>>,
}

/// A face ready to draw with, cheap to hand to whoever draws next.
///
/// The two defaults are process-wide statics and everything else was opened
/// for this render; a caller wants neither distinction, only a font.
#[derive(Debug, Clone)]
pub(crate) enum Typeface {
    /// `sans` or `serif` at the weight every older document meant.
    Shipped(&'static Font),
    /// Anything else, opened once and shared.
    Opened(Arc<Font>),
}

impl Deref for Typeface {
    type Target = Font;

    fn deref(&self) -> &Font {
        match self {
            Self::Shipped(font) => font,
            Self::Opened(font) => font,
        }
    }
}

/// Which face, at which weight — everything that decides whether two text
/// assets can share one parsed instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Face {
    /// One scorsese ships, at a weight and a slant.
    Shipped(String, u16, Slant),
    /// One the project carries.
    File(PathBuf, Option<u16>),
}

impl Painter {
    /// Everything needed to set `asset` as `clip` shows it, at any instant.
    ///
    /// What a text layer keeps for the whole segment: drawn once and held when
    /// nothing about it changes, and drawn from again on every frame when it
    /// reveals or counts. Where the block sits is what the clip's anchor says,
    /// and moving it from there is `transform.position.*` like any other layer.
    pub(crate) fn typing(
        &mut self,
        asset: &Asset,
        clip: &Clip,
        project_root: &Path,
        resolution: Resolution,
    ) -> Result<Typing, RenderError> {
        let style = asset.text_style();
        let font = self.font(&style, asset, project_root)?;
        Ok(Typing::new(
            asset,
            clip,
            font,
            resolve(&style, clip.anchor, resolution),
        ))
    }

    /// Where this asset's words would be set, without setting them.
    ///
    /// The **wrapped block**, which is the rectangle the anchor already reasons
    /// about — so an arrow attached to a title meets the words rather than the
    /// frame. Same style, same font, same layout as [`Typing::draw`]; the two
    /// cannot disagree because they share the step that decides it.
    pub(crate) fn block(
        &mut self,
        asset: &Asset,
        clip: &Clip,
        project_root: &Path,
        resolution: Resolution,
    ) -> Result<Area, RenderError> {
        let typing = self.typing(asset, clip, project_root, resolution)?;
        Ok(typing.block(resolution))
    }

    /// The face a style names, at the weight it names, opening and keeping a
    /// project's own font file the first time it is asked for.
    ///
    /// A reserved name with no weight comes back as the compositor's own
    /// `&'static`, which is the common case and costs nothing. A weight beside
    /// one is an ordinary instance of a variable face and is cached like any
    /// other — and whether the face's axis actually reaches that weight is a
    /// fact about bytes, so it is refused here rather than at validation, the
    /// same as for a file the project carries.
    fn font(
        &mut self,
        style: &TextStyle,
        asset: &Asset,
        project_root: &Path,
    ) -> Result<Typeface, RenderError> {
        let key = match (&style.font, style.weight) {
            // The two names every project written before this one uses, at the
            // weight they have always meant. Answered from the compositor's own
            // statics, so the common case allocates nothing.
            (FontChoice::Named(name), None) if name == "sans" && !style.italic => {
                return Ok(Typeface::Shipped(Font::sans()));
            }
            (FontChoice::Named(name), None) if name == "serif" && !style.italic => {
                return Ok(Typeface::Shipped(Font::serif()));
            }
            (FontChoice::Named(name), weight) => Face::Shipped(
                name.clone(),
                weight.unwrap_or(text::SHIPPED_WEIGHT),
                slant(style.italic),
            ),
            // A file is one drawing and has no second table to reach for, so
            // the field is refused rather than quietly dropped. The message
            // says the thing to do instead, which is to name the italic file.
            (FontChoice::File(path), _) if style.italic => {
                return Err(RenderError::UnusableFont {
                    asset: asset.id.to_string(),
                    path: path.resolve(project_root),
                    detail: "`italic` applies to a font scorsese ships, which carries its \
                             italic beside its upright. A font the project carries is one \
                             file — name the italic file itself instead"
                        .to_owned(),
                });
            }
            (FontChoice::File(path), weight) => Face::File(path.resolve(project_root), weight),
        };
        if !self.fonts.contains_key(&key) {
            let font = open(&key, asset)?;
            self.fonts.insert(key.clone(), Arc::new(font));
        }
        Ok(Typeface::Opened(Arc::clone(&self.fonts[&key])))
    }
}

/// Turns the document's fractions into the pixels the compositor draws in.
///
/// Size is a fraction of the frame's **height** and width a fraction of its
/// **width**: the height, because that is what makes a line of text the same
/// proportion of the picture at any aspect ratio, and a wrap column measured
/// down rather than across would be a surprise to everyone.
fn resolve(
    style: &TextStyle,
    anchor: Anchor,
    resolution: scorsese_compositor::Resolution,
) -> Style {
    let height = f64::from(resolution.height());
    let width = f64::from(resolution.width());
    let size = style.size * height;
    Style {
        size: size as f32,
        color: style.color,
        align: style.align,
        line_height: (size * style.line_height) as f32,
        max_width: (style.max_width * width) as f32,
        // A rim is a thickness, and a thickness has no axis of its own — so it
        // takes the height, the same one `size` takes and the same one a
        // shape's border takes. A caption is the same weight of edge at 4:3 and
        // at 16:9, which the eight-offset-copies workaround this replaces could
        // never be.
        edge: style.stroke.map(|color| Edge {
            color,
            width: (style.stroke_width * height) as f32,
        }),
        // The anchor comes off the **clip**, not the style: it is where this
        // placement of the text sits, and the same text asset used twice may
        // legitimately sit in two different corners.
        anchor,
        // A counting figure is set tabular so its line holds still; a text
        // without one keeps the face's own figures, as it always has.
        figures: if style.number.is_some() {
            Figures::Tabular
        } else {
            Figures::Proportional
        },
    }
}

/// The document's boolean as the compositor's named pair.
const fn slant(italic: bool) -> Slant {
    if italic {
        Slant::Italic
    } else {
        Slant::Upright
    }
}
