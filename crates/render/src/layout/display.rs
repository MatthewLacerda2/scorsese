//! How an answer about where things are reads.
//!
//! The reader is usually an agent about to write a number back into the
//! document, so every figure on the page is in the unit it would write —
//! fractions of the frame, three decimals, which is a tenth of a pixel at
//! 1080p and finer than anything anyone is placing by hand.
//!
//! The clips with no rectangle are gathered at the foot rather than listed one
//! per line, because there are usually many of them and each says the same
//! thing: a film's other forty clips are simply somewhere else on the timeline.

use std::fmt;

use crate::describe::display::kind;

use super::{Absence, Layout, Placement, Region, Unplaced};

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "where things land at {:.2}s (frame {}), on a {} frame — as fractions of it",
            self.at.seconds,
            self.at.frame.get(),
            self.raster
        )?;
        if self.placed.is_empty() {
            write!(f, "\n  nothing is on screen here")?;
        }
        // Both columns sized to what is actually in them, so the rectangles
        // line up for a reader scanning down them rather than reading across.
        // `generated_video` is more than twice the width of `text`, and a
        // column fixed at either would ruin the other.
        let width = self.widest(|placement| placement.named().chars().count());
        let named = self.widest(|placement| kind(placement.kind).len());
        for placement in &self.placed {
            write!(
                f,
                "\n  {:<width$}  {:<named$}  {}",
                placement.named(),
                kind(placement.kind),
                placement.area
            )?;
        }
        for unplaced in &self.unplaced {
            if let Absence::Unknown(why) = &unplaced.why {
                write!(f, "\n  no rectangle for {} — {why}", unplaced.named())?;
            }
        }
        listed(f, "not on screen here", self, &Absence::OffScreen)?;
        listed(f, "in the mix, not on the picture", self, &Absence::Sound)
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "x {:.3}–{:.3}  y {:.3}–{:.3}  ({:.3} × {:.3})",
            self.left,
            self.right(),
            self.top,
            self.bottom(),
            self.width,
            self.height
        )
    }
}

impl Layout {
    /// How wide a column of `measure` has to be to hold every placed layer.
    fn widest(&self, measure: impl Fn(&Placement) -> usize) -> usize {
        self.placed.iter().map(measure).max().unwrap_or(0)
    }
}

impl Placement {
    /// The clip as the description names it: which lane, then which clip.
    fn named(&self) -> String {
        format!("{}/{}", self.track, self.clip)
    }
}

impl Unplaced {
    /// The same, so a clip reads the same way whether it has a rectangle here
    /// or not.
    fn named(&self) -> String {
        format!("{}/{}", self.track, self.clip)
    }
}

/// One line naming every clip absent for the same reason, or nothing at all
/// when none is.
fn listed(f: &mut fmt::Formatter<'_>, lead: &str, layout: &Layout, why: &Absence) -> fmt::Result {
    let clips: Vec<&str> = layout
        .unplaced
        .iter()
        .filter(|unplaced| &unplaced.why == why)
        .map(|unplaced| unplaced.clip.as_str())
        .collect();
    if clips.is_empty() {
        return Ok(());
    }
    write!(f, "\n  {lead}: {}", clips.join(", "))
}
