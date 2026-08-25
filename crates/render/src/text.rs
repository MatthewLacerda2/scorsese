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

use scorsese_compositor::text::{self, Edge, Font, Slant, Style, Unpaintable};
use scorsese_compositor::{Area, Frame, Resolution};
use scorsese_core::{Anchor, Asset, AssetKind, FontChoice, Project, TextStyle};

use crate::error::RenderError;

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
    fonts: HashMap<Face, Font>,
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
    /// Draws `asset`'s text across the whole of `frame`.
    ///
    /// The frame is cleared to transparent first: a text layer is glyphs and
    /// nothing else, so everywhere the letters are not, the tracks underneath
    /// show through. Where the block sits is what `anchor` says — the frame's
    /// centre unless the clip asked otherwise — and moving it from there is
    /// `transform.position.*` like any other layer.
    ///
    /// What comes back is whatever a colour glyph in the content asked for and
    /// could not be given — empty for text made of letters, which is nearly all
    /// of it. The caller turns each into a note, because a glyph drawn short
    /// with nothing saying so is the failure this whole path exists to end.
    pub(crate) fn paint(
        &mut self,
        frame: &mut Frame,
        asset: &Asset,
        anchor: Anchor,
        project_root: &Path,
    ) -> Result<Vec<Unpaintable>, RenderError> {
        let style = asset.text_style();
        let content = asset.text.clone().unwrap_or_default();
        let resolution = frame.resolution();
        let font = self.font(&style, asset, project_root)?;

        frame.fill_transparent();
        Ok(text::draw(
            frame,
            &content,
            font,
            &resolve(&style, anchor, resolution),
        ))
    }

    /// Where this asset's words would be set, without setting them.
    ///
    /// The **wrapped block**, which is the rectangle the anchor already reasons
    /// about — so an arrow attached to a title meets the words rather than the
    /// frame. Same style, same font, same layout as [`Painter::paint`]; the two
    /// cannot disagree because they share the step that decides it.
    pub(crate) fn block(
        &mut self,
        asset: &Asset,
        anchor: Anchor,
        project_root: &Path,
        resolution: Resolution,
    ) -> Result<Area, RenderError> {
        let style = asset.text_style();
        let content = asset.text.clone().unwrap_or_default();
        let font = self.font(&style, asset, project_root)?;
        Ok(text::block_in(
            &content,
            font,
            &resolve(&style, anchor, resolution),
            text::Band::whole(resolution),
            resolution,
        ))
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
    ) -> Result<&Font, RenderError> {
        let key = match (&style.font, style.weight) {
            // The two names every project written before this one uses, at the
            // weight they have always meant. Answered from the compositor's own
            // statics, so the common case allocates nothing.
            (FontChoice::Named(name), None) if name == "sans" && !style.italic => {
                return Ok(Font::sans());
            }
            (FontChoice::Named(name), None) if name == "serif" && !style.italic => {
                return Ok(Font::serif());
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
    }
}

/// A text asset naming a font this build does not ship.
///
/// A **problem** rather than a warning, unlike an unknown keyframe property:
/// a track nothing animates leaves a render that is merely missing a fade,
/// where a face nothing can find leaves no render at all. So it is worth
/// finding before an encode starts rather than partway through one, which is
/// the whole reason `scorsese check` exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFont {
    /// The text asset naming it.
    pub asset: String,
    /// The name as authored, quoted back so it can be searched for.
    pub named: String,
    /// Every name this build does answer to — the question being asked at the
    /// moment somebody reads this.
    pub available: String,
}

impl std::fmt::Display for UnknownFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "there is no font called `{}`. The ones scorsese ships are: {}",
            self.named, self.available
        )
    }
}

/// Every text asset in the project naming a shipped face that does not exist.
///
/// Only names: a `font` that is a **path** is a file on disk, and whether it is
/// there is `check`'s media pass to answer, alongside every other missing file.
pub fn unknown_fonts(project: &Project) -> Vec<UnknownFont> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Text)
        .filter_map(|asset| {
            let named = asset.text_style().font.name()?.to_owned();
            text::family(&named).is_none().then(|| UnknownFont {
                asset: asset.id.to_string(),
                named,
                available: text::names().collect::<Vec<_>>().join(", "),
            })
        })
        .collect()
}

/// A text asset saying something **no face scorsese has** can draw.
///
/// A warning and never a problem: the render succeeds, the frames are fine
/// everywhere else, and swapping the face or the character is the author's
/// call rather than this command's. What makes it worth saying at all is that
/// the alternative to saying it is finding out by eye — an unmapped character
/// is dropped **with its advance**, so the line closes up and looks like text
/// nobody wrote rather than text that failed.
///
/// The named face is not the whole question, since font fallback arrived: a
/// character it lacks and the emoji face draws reaches the frame, and objecting
/// to it would be a warning about something correct. What is left here is what
/// still vanishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredGlyphs {
    /// The text asset whose content it is.
    pub asset: String,
    /// The face as authored — a shipped name or a path — so the answer is
    /// "swap this" rather than "swap something".
    pub face: String,
    /// The characters it cannot draw, each named once, in the order they
    /// first appear.
    pub characters: Vec<char>,
}

impl std::fmt::Display for UncoveredGlyphs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The code point beside the character, because half of these are
        // invisible in a terminal and `U+2713` is the searchable half.
        let listed = self
            .characters
            .iter()
            .map(|character| format!("`{character}` (U+{:04X})", *character as u32))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "nothing scorsese can set `{}` in has a glyph for {listed} — they \
             are dropped, not drawn",
            self.face
        )
    }
}

/// Every text asset saying something no face in its chain can draw.
///
/// Resolves each asset's face exactly as a render would, through the same
/// painter, so this can never disagree with what the frames do. A face that
/// will not open at all is not reported here — that is `unknown_fonts` above,
/// or the media pass, and reporting it twice would be two findings for one
/// fault.
pub fn uncovered_glyphs(project: &Project, project_root: &Path) -> Vec<UncoveredGlyphs> {
    let mut painter = Painter::default();
    let mut found = Vec::new();
    for asset in project.assets.iter().filter(|a| a.kind == AssetKind::Text) {
        let Some(content) = asset.text.as_ref() else {
            continue;
        };
        let style = asset.text_style();
        let Ok(font) = painter.font(&style, asset, project_root) else {
            continue;
        };
        let characters = font.uncovered(content);
        if characters.is_empty() {
            continue;
        }
        found.push(UncoveredGlyphs {
            asset: asset.id.to_string(),
            face: face_named(&style.font),
            characters,
        });
    }
    found
}

/// The face as the document wrote it — a shipped name, or the path the
/// project carries. `FontChoice` is serialised as a plain string either way,
/// which is the same string an author would search for.
fn face_named(choice: &FontChoice) -> String {
    match choice {
        FontChoice::Named(name) => name.clone(),
        FontChoice::File(path) => path.to_string(),
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
