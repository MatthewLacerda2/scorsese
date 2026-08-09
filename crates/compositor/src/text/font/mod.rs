//! The faces that can be drawn with, and the two that ship.
//!
//! **The fonts are committed to the repository**, in `crates/compositor/fonts/`
//! with the SIL Open Font License beside them. That is forced rather than
//! preferred: a system-font lookup resolves to a different file on Linux, macOS
//! and Windows, so a golden reference blessed on one machine would fail on
//! another and the pixel gate would become noise. Deterministic text means a
//! font we ship.
//!
//! Inter and Source Serif 4 are the two, both as **one variable file each**
//! covering their whole weight range — which is what lets `weight` mean
//! something on a fresh install rather than only on a font somebody went and
//! fetched. Which release of each, which axes it carries, and what every axis
//! other than `wght` is therefore left at, are recorded in
//! `crates/compositor/fonts/README.md`.
//!
//! They are defaults, not the vocabulary: [`Font::from_bytes`] takes any font
//! file a project brings with it.
//!
//! What is read out of a face is deliberately narrow — a glyph for a character,
//! its advance, and the outline to fill. Kerning and the rest of shaping are a
//! different problem with a different crate behind it, and a title card does
//! not have it.
//!
//! **Weight is the one exception, and it is here because silence was the bug.**
//! Most modern open faces now ship *only* as variable files — one file holding
//! a continuous `wght` axis — and every one of them declares a default instance
//! that is whatever the designer chose. Manrope's is 200; Outfit's is 100.
//! Reading such a file and drawing with `LocationRef::default()` sets a title
//! card in hairline Thin and reports nothing, which is a shot rendered wrong
//! that no error mentions. So the axis is not something this module may ignore:
//! [`Font::from_bytes`] takes the weight alongside the bytes, and the
//! combinations that cannot mean anything are refused rather than guessed at.
//! Only `wght` is read — `opsz`, `wdth` and `slnt` are real axes and none of
//! them is what "make this bold" means.
//!
//! What is read out of a face is deliberately narrow — how tall it sets, the
//! outline of a glyph, and a shaper to choose and place those glyphs (see
//! [`super::shape`]). Two crates read the one face: skrifa for the outlines,
//! HarfRust for the shaping. They are versioned to share a `read-fonts`
//! underneath, so what they are handed is the same parsed [`FontRef`] and not
//! two of them — the reasoning is in `crates/compositor/Cargo.toml`.

use std::fmt;
use std::sync::OnceLock;

use harfrust::{Shaper, ShaperData, ShaperInstance};
use skrifa::instance::{Location, LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use super::shape::{self, Shaped};

mod shipped;
mod weight;

pub use shipped::{Cut, Family, SHIPPED, family, names};
use weight::locate;

/// What a shipped face is drawn at when the document names no weight.
///
/// Regular, and it is the whole of the compatibility story. The general rule is
/// that a **variable file with no weight is refused**, because the file's own
/// `fvar` default is 200 in Manrope and 100 in Outfit and "it will be Regular"
/// is not a safe guess about a file nobody here has seen. That reasoning is
/// about a file scorsese cannot know. It does not apply to a face scorsese
/// ships and whose axis it publishes — so these two default, and every document
/// ever written against `"font": "sans"` goes on rendering at the weight it
/// always did.
pub const SHIPPED_WEIGHT: u16 = 400;

/// A face, checked and ready to draw with.
///
/// Holds the file rather than a parsed view of it: the parsed view borrows the
/// bytes, and a struct holding both would be self-referential. Re-reading the
/// table directory is a handful of bounds checks, paid once per drawn block
/// rather than per glyph.
///
/// The shaper's own view of the face is the exception, and it is kept here for
/// the opposite reason: working out which of the font's lookups a feature
/// reaches is real work, it borrows nothing, and it does not depend on the
/// size a block is set at — so it is done once for the file and reused by
/// every block, every line and every frame that face ever sets.
pub struct Font {
    bytes: Vec<u8>,
    /// Where in variation space to draw. Empty for a static file, and one
    /// normalised coordinate per `fvar` axis for a variable one — computed
    /// once when the face is made, because it is the same for every glyph.
    location: Location,
    shaping: ShaperData,
    /// The same variation position as `location`, in the form the shaper
    /// wants. Built once, beside it, because a `Shaper` borrows its instance
    /// and one made per call could not outlive the call.
    instance: ShaperInstance,
}

impl Font {
    /// The shipped sans face at [`SHIPPED_WEIGHT`], read once for the process.
    ///
    /// The common case and the cheap one: no weight named is what every
    /// document said before there was a weight to name, and what a slug card, a
    /// contact sheet's labels and a ruled grid all want. A weight named on top
    /// of it goes through [`Font::shipped`], which builds a face rather than
    /// borrowing this one.
    pub fn sans() -> &'static Self {
        static FONT: OnceLock<Font> = OnceLock::new();
        FONT.get_or_init(|| Self::compiled_in("sans"))
    }

    /// The shipped serif face, under the same rule as [`Font::sans`].
    pub fn serif() -> &'static Self {
        static FONT: OnceLock<Font> = OnceLock::new();
        FONT.get_or_init(|| Self::compiled_in("serif"))
    }

    /// A shipped family, by the name a document wrote, at the weight it asked
    /// for.
    ///
    /// `None` means [`SHIPPED_WEIGHT`], so this and [`Font::sans`] agree about
    /// what an unweighted `sans` is.
    ///
    /// Three refusals, and each one is a different question:
    ///
    /// - **no such family** — this build ships none by that name, and the
    ///   message carries the ones it does, because that is the question being
    ///   asked whenever somebody gets a name wrong;
    /// - **a variable family that does not reach the weight** — refused with
    ///   the range it does reach, since clamping is a silent substitution;
    /// - **a drawn family that was not drawn at it** — refused with the weights
    ///   it *was* drawn at. Liberation has Regular and Bold and nothing
    ///   between, and snapping 600 to 700 would be the same substitution one
    ///   step along.
    pub fn shipped(name: &str, weight: Option<u16>) -> Result<Self, FontError> {
        let family = shipped::family(name).ok_or_else(|| FontError::NoSuchFamily {
            name: name.to_owned(),
            available: shipped::names().collect::<Vec<_>>().join(", "),
        })?;
        let wanted = weight.unwrap_or(SHIPPED_WEIGHT);
        match family.cut {
            // The axis answers, and refuses what it does not reach.
            Cut::Variable(bytes) => Self::from_bytes(bytes, Some(wanted)),
            // No axis to ask, so the table is the whole answer. `None` on the
            // way in because each of these files is static and naming a weight
            // at a static file is itself refused.
            Cut::Drawn(files) => files
                .iter()
                .find(|(drawn, _)| *drawn == wanted)
                .map_or_else(
                    || {
                        Err(FontError::WeightNotDrawn {
                            weight: wanted,
                            family: family.family.to_owned(),
                            drawn: family
                                .drawn_weights()
                                .iter()
                                .map(u16::to_string)
                                .collect::<Vec<_>>()
                                .join(", "),
                        })
                    },
                    |(_, bytes)| Self::from_bytes(bytes, None),
                ),
        }
    }

    /// Parses a font file a project brought with it — a TrueType or OpenType
    /// face, as bytes — set at `weight` if it is variable.
    ///
    /// Bytes rather than a path: this crate does no file I/O, so opening the
    /// file belongs to whoever knows where the project root is. Parsing here
    /// rather than at the first glyph means a bad file is a refusal before a
    /// render starts instead of a panic part way through one.
    ///
    /// **Weight and file have to agree, and disagreeing is an error rather
    /// than a preference.** A variable file with no weight named is refused,
    /// because the alternative is drawing at a default the document never
    /// chose and never mentioned — the file's `fvar` default is 200 in Manrope
    /// and 100 in Outfit, so "it will be Regular" is not a safe guess anywhere.
    /// A weight the axis does not reach is refused with the range it does
    /// reach, since clamping 900 to 800 would be a second silent substitution.
    /// A static file with a weight named is refused too: it has exactly one
    /// weight, and quietly ignoring the field is how someone comes to insist
    /// their bold is broken.
    pub fn from_bytes(bytes: &[u8], weight: Option<u16>) -> Result<Self, FontError> {
        let font = FontRef::new(bytes).map_err(|error| FontError::Unreadable(error.to_string()))?;
        let location = locate(&font, weight)?;
        Ok(Self {
            instance: ShaperInstance::from_coords(&font, location.coords().iter().copied()),
            shaping: ShaperData::new(&font),
            bytes: bytes.to_vec(),
            location,
        })
    }

    /// The face set at one size, which is the form everything else here wants.
    pub(super) fn at(&self, size: f32) -> Face<'_> {
        let font = FontRef::new(&self.bytes).expect("these bytes parsed when the font was made");
        let at = Size::new(size);
        let location = LocationRef::from(&self.location);
        let line = font.metrics(at, location);
        // The shaper is pinned to the **same** instance the outlines are drawn
        // at. Shaping a variable face at its default while filling it at 700
        // would kern a bold in the regular's metrics — the letters would be the
        // weight asked for and the spacing between them would not.
        let shaper = self
            .shaping
            .shaper(&font)
            .instance(Some(&self.instance))
            .build();
        // Shaping reports in font units, because a shaped run is the same run
        // whatever it is set at; everything downstream of here is in pixels of
        // the raster, so the ratio is worked out once per face rather than per
        // glyph. A face claiming no units per em cannot be scaled sanely, and
        // one that sets nothing is better than one that sets a smear.
        let units = shaper.units_per_em().max(1) as f32;
        Face {
            glyphs: font.outline_glyphs(),
            shaper,
            scale: size / units,
            ascent: line.ascent,
            descent: line.descent,
            at,
            location: self.location.clone(),
        }
    }

    /// Reads a face this build ships. Its bytes are compiled in, so failing
    /// here would mean a corrupt binary rather than a bad project.
    fn compiled_in(name: &str) -> Self {
        Self::shipped(name, None)
            .expect("a font compiled into this binary parses, and reaches its own default weight")
    }
}

impl fmt::Debug for Font {
    /// The file itself is hundreds of kilobytes and nothing a reader wants in
    /// a log line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font").finish_non_exhaustive()
    }
}

/// One face at one size: everything measuring and drawing needs, looked up
/// once instead of per character.
pub(super) struct Face<'a> {
    glyphs: OutlineGlyphCollection<'a>,
    shaper: Shaper<'a>,
    /// Font units to pixels, at the size this face was taken at.
    scale: f32,
    ascent: f32,
    descent: f32,
    at: Size,
    /// Cloned from the face rather than borrowed, so drawing a glyph needs
    /// nothing but this struct. It is a handful of coordinates.
    location: Location,
}

impl Face<'_> {
    /// Which glyphs set `text` and where each one goes, kerning applied.
    ///
    /// The only way to a width in this module: measuring a string any other
    /// way would answer with the spacing the face did *not* ask for, and
    /// wrapping would then break lines in places the drawn text does not.
    pub(super) fn shape(&self, text: &str) -> Shaped {
        shape::shape(&self.shaper, text, self.scale)
    }

    /// How far the tallest glyph reaches above the baseline and the lowest
    /// below it. The descent is negative, as the face states it.
    pub(super) fn extents(&self) -> (f32, f32) {
        (self.ascent, self.descent)
    }

    /// Walks the outline of glyph `id` through `pen`, in font-space with **y
    /// upwards** — the convention outlines are written in, which whoever draws
    /// them has to flip onto a raster.
    pub(super) fn outline(&self, id: GlyphId, pen: &mut impl OutlinePen) {
        let Some(glyph) = self.glyphs.get(id) else {
            return;
        };
        // A glyph that will not draw — a corrupt outline in an otherwise
        // readable file — leaves a gap of its own width rather than stopping
        // the render. The rest of the title is still worth having.
        let _ = glyph.draw(
            DrawSettings::unhinted(self.at, LocationRef::from(&self.location)),
            pen,
        );
    }
}

/// Why a font file could not be used.
///
/// Not `Eq`: an axis states its bounds as the floats the `fvar` table holds,
/// and rounding them to say they compare would be inventing a fact about the
/// file.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FontError {
    /// Not a font this build can read: the wrong kind of file, a truncated
    /// one, or a format the parser does not know.
    #[error("this is not a font scorsese can read: {0}")]
    Unreadable(
        /// What the parser said, in its own words.
        String,
    ),

    /// A variable font, and nothing said about how heavy to set it.
    ///
    /// The message names the file's own default because that is the weight
    /// that *would* have been used, and it is the number a reader needs in
    /// order to see the trap: a file whose default is 200 sets a title in
    /// hairline Thin while looking entirely correct.
    #[error(
        "this font is variable — it has a `wght` axis from {min} to {max}, defaulting to \
         {default} — so `style.weight` has to say which weight to set it at"
    )]
    VariableWithoutWeight {
        /// The lightest weight the file offers.
        min: f32,
        /// The instance the file would fall back to, which is exactly what is
        /// not being allowed to happen silently.
        default: f32,
        /// The heaviest weight the file offers.
        max: f32,
    },

    /// A name no family in this build answers to.
    ///
    /// The list goes in the message rather than being left to be looked up,
    /// because "which fonts are there?" is the question somebody is asking at
    /// the moment they see this, and a refusal that does not answer it sends
    /// them to the documentation for one line.
    #[error("there is no font called `{name}`. The ones scorsese ships are: {available}")]
    NoSuchFamily {
        /// The name the document wrote.
        name: String,
        /// Every name this build answers to, aliases included.
        available: String,
    },

    /// A weight a **drawn** family was not drawn at.
    ///
    /// Distinct from [`FontError::WeightOffAxis`] because the answer is a list
    /// rather than a range: there is no axis to be off, only the weights the
    /// designer actually drew, and 600 is not one of them however close it
    /// looks to 700.
    #[error(
        "{family} is drawn at {drawn} — it has no weight {weight}, and there is no axis \
             between them to interpolate one from"
    )]
    WeightNotDrawn {
        /// The weight asked for.
        weight: u16,
        /// The family, by its own name.
        family: String,
        /// The weights it does have.
        drawn: String,
    },

    /// A weight this file's axis does not reach. Refused rather than clamped:
    /// clamping 900 to 800 is the same silent substitution the whole rule
    /// exists to stop, one step further along.
    #[error("this font's `wght` axis runs from {min} to {max}, so it has no weight {weight}")]
    WeightOffAxis {
        /// The weight asked for.
        weight: u16,
        /// The lightest weight the file offers.
        min: f32,
        /// The heaviest weight the file offers.
        max: f32,
    },

    /// A weight named for a file that has only one. Ignoring it would look
    /// exactly like honouring it.
    ///
    /// The message says "no `wght` axis" rather than "static", because that is
    /// the fact and it is very slightly narrower: a face varying only on
    /// `opsz` or `wdth` is a variable file with one weight, and telling its
    /// author it is static would send them looking for the wrong thing.
    #[error("this font has no `wght` axis — it has one weight — so it cannot be set at {weight}")]
    StaticWithWeight {
        /// The weight that could not be honoured.
        weight: u16,
    },
}
