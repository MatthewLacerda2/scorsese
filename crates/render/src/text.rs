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
//! ordinary layer. There is no text path through the renderer beyond this file:
//! a title is composited, transformed and faded by exactly the code a video
//! clip goes through.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use scorsese_compositor::Frame;
use scorsese_compositor::text::{self, Font, Style};
use scorsese_core::{Anchor, Asset, FontChoice, TextStyle};

use crate::error::RenderError;

/// Draws text assets, holding on to the fonts it has opened.
///
/// Kept for a whole render rather than made per clip: parsing a face costs
/// milliseconds, and a cut with a title on every shot would otherwise pay that
/// for each one. The two shipped faces are the compositor's and are parsed once
/// per process; only a project's own font lands in this map.
///
/// **Keyed by file *and* weight**, because one variable file is many faces: a
/// project setting its titles in Manrope at 800 and its captions in the same
/// file at 400 has two instances, and they must not share a cache slot. That
/// is the whole point of the feature — one file where the per-weight file tax
/// used to be — so the cache has to be able to hold both at once.
#[derive(Debug, Default)]
pub struct Painter {
    fonts: HashMap<(PathBuf, Option<u16>), Font>,
}

impl Painter {
    /// One with nothing opened yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws `asset`'s text across the whole of `frame`.
    ///
    /// The frame is cleared to transparent first: a text layer is glyphs and
    /// nothing else, so everywhere the letters are not, the tracks underneath
    /// show through. Where the block sits is what `anchor` says — the frame's
    /// centre unless the clip asked otherwise — and moving it from there is
    /// `transform.position.*` like any other layer.
    pub fn paint(
        &mut self,
        frame: &mut Frame,
        asset: &Asset,
        anchor: Anchor,
        project_root: &Path,
    ) -> Result<(), RenderError> {
        let style = asset.text_style();
        let content = asset.text.clone().unwrap_or_default();
        let resolution = frame.resolution();
        let font = self.font(&style, asset, project_root)?;

        frame.fill_transparent();
        text::draw(frame, &content, font, &resolve(&style, anchor, resolution));
        Ok(())
    }

    /// The face a style names, at the weight it names, opening and keeping a
    /// project's own font file the first time it is asked for.
    ///
    /// The two reserved names come back unweighted, and a `weight` beside one
    /// of them never reaches here: the shipped faces are static and known to be
    /// static without opening anything, so [`Project::validate`] refuses that
    /// pairing before a render can start. Everything a *file* has to say about
    /// its own weights — whether it is variable at all, and how far its axis
    /// runs — is a fact about bytes on disk, so the refusal lives here, where
    /// the bytes are.
    ///
    /// [`Project::validate`]: scorsese_core::Project::validate
    fn font(
        &mut self,
        style: &TextStyle,
        asset: &Asset,
        project_root: &Path,
    ) -> Result<&Font, RenderError> {
        let path = match &style.font {
            FontChoice::Sans => return Ok(Font::sans()),
            FontChoice::Serif => return Ok(Font::serif()),
            FontChoice::File(path) => path.resolve(project_root),
        };
        let key = (path, style.weight);
        if !self.fonts.contains_key(&key) {
            let unusable = |detail: String| RenderError::UnusableFont {
                asset: asset.id.to_string(),
                path: key.0.clone(),
                detail,
            };
            let bytes = std::fs::read(&key.0).map_err(|source| unusable(source.to_string()))?;
            let font = Font::from_bytes(&bytes, style.weight)
                .map_err(|error| unusable(error.to_string()))?;
            self.fonts.insert(key.clone(), font);
        }
        Ok(&self.fonts[&key])
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
        // The anchor comes off the **clip**, not the style: it is where this
        // placement of the text sits, and the same text asset used twice may
        // legitimately sit in two different corners.
        anchor,
    }
}
