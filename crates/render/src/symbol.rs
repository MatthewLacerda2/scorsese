//! Turning an icon asset into a layer's worth of pixels.
//!
//! The same one thing happens here that happens in [`crate::shape`] and nowhere
//! else: **fractions become pixels**. A project stores a symbol's size as a
//! fraction of the frame so that one document reads the same at 720p and at 4K,
//! and the raster it is a fraction *of* is a render setting, known here.
//!
//! Called `symbol` rather than `icon` because `icon` is already taken at this
//! crate's root, by the re-export of the catalogue itself — and that is the
//! honest division of the two: [`scorsese_compositor::icon`] is the set of
//! symbols and the drawing of one, this is the seam where a document reaches it.
//!
//! **Two units meet here and they are not the same one, which is the whole
//! reason this file is worth reading.** `size` is a fraction of the raster's
//! height, so an icon is measured against the frame like a title. `stroke_width`
//! is a fraction of the *icon's own box*, so it is measured against the icon and
//! scales with it — the opposite of a shape's border, which stays the same
//! visible weight however big the box is. A border wants that and a symbol does
//! not: a shrunk icon with a raster-relative stroke closes its counters up and
//! reads as a blob.

use std::fmt;

use scorsese_compositor::icon::{self, Symbol};
use scorsese_compositor::{Area, Frame, Resolution};
use scorsese_core::{Anchor, AssetKind, Icon, Project};

/// Draws `icon` onto `frame`, sized against the frame's own raster.
///
/// The frame is cleared to transparent first, exactly as a shape's is: an icon
/// layer is the symbol and nothing else, and everywhere it is not — including
/// the space inside its own strokes — the tracks below have to show through.
///
/// A name the set does not have draws **nothing**, and the layer stays
/// transparent. Refusing the render outright would make one mistyped string
/// cost a whole preview; the render says so in its report instead, and
/// `scorsese check` says so before an encode is ever started.
pub(crate) fn paint(frame: &mut Frame, icon: &Icon, anchor: Anchor) {
    frame.fill_transparent();
    let Some(symbol) = symbol(icon, frame.resolution(), anchor) else {
        return;
    };
    scorsese_compositor::icon::draw(frame, &symbol);
}

/// Whether this build has no symbol of that name.
///
/// Its own question rather than reading it off an absent [`area`], because the
/// two absences are different: an area is also missing for a size that could
/// not describe a square, and a render that reported *unknown name* for that
/// would be sending whoever reads it to look at the wrong field.
pub(crate) fn is_unknown(icon: &Icon) -> bool {
    scorsese_compositor::icon::find(&icon.name).is_none()
}

/// Where this icon's own square lands on the raster, in pixels.
///
/// `None` for a name that is not in the set or a size that could not describe a
/// square. This is the rectangle an attached arrow meets: what a reader sees,
/// rather than the raster-sized layer the symbol happens to be drawn into.
pub(crate) fn area(icon: &Icon, resolution: Resolution, anchor: Anchor) -> Option<Area> {
    icon::area_of(&symbol(icon, resolution, anchor)?, resolution)
}

/// The icon as the compositor takes it: the same symbol, in pixels.
///
/// `None` when the name is not one this build ships, which is the one thing
/// about an icon asset that cannot be answered from the document alone.
fn symbol(icon: &Icon, resolution: Resolution, anchor: Anchor) -> Option<Symbol> {
    let size = (icon.size * f64::from(resolution.height())) as f32;
    Some(Symbol {
        icon: icon::find(&icon.name)?,
        size,
        anchor,
        color: icon.color,
        // Against the icon's own box, not the raster — see the module doc.
        width: (icon.stroke_width as f32) * size,
    })
}

/// An icon asset naming a symbol this build does not ship.
///
/// A **problem** rather than a warning, and the same one an unknown font is: a
/// name nothing can find leaves a hole in the picture where a symbol was meant
/// to be, and a hole is invisible to anything counting frames. Worth finding
/// before an encode starts rather than by watching the result, which is the
/// whole reason `scorsese check` exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownIcon {
    /// The icon asset naming it.
    pub asset: String,
    /// The name as authored, quoted back so it can be searched for.
    pub named: String,
    /// The names that are close to it, best first. Empty when nothing is —
    /// which is itself the answer, and reads as one.
    pub nearest: Vec<&'static str>,
}

impl fmt::Display for UnknownIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "there is no icon called `{}`", self.named)?;
        // The available set cannot be listed the way the shipped faces can —
        // there are seventeen hundred of them — so the near ones are what a
        // refusal has to offer instead.
        if self.nearest.is_empty() {
            return Ok(());
        }
        let listed = self
            .nearest
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, ". Did you mean {listed}?")
    }
}

/// Every icon asset in the project naming a symbol that does not exist.
pub fn unknown_icons(project: &Project) -> Vec<UnknownIcon> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Icon)
        .filter_map(|asset| {
            let named = asset.icon.as_ref()?.name.clone();
            icon::find(&named).is_none().then(|| UnknownIcon {
                asset: asset.id.to_string(),
                nearest: icon::nearest(&named),
                named,
            })
        })
        .collect()
}
