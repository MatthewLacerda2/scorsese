//! How large a raster is, given the shape it must be (#1278).
//!
//! A project carries its frame's shape ([`scorsese_core::Ratio`]); how many
//! pixels it has is chosen by whoever is drawing it, by what the picture is
//! for. [`Resolution::framed`] is the one answer to *the frame for this project
//! at this budget*, and every surface — a render's default size, a still's,
//! the frame `describe` and the checkup measure against, a sheet's cells —
//! asks it rather than writing its own size down.

use std::fmt;
use std::str::FromStr;

use scorsese_core::{Project, Ratio};

use crate::frame::{Resolution, ResolutionError};

/// What a raster is for, which decides how many pixels it spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Budget {
    /// A delivered file, or a PNG somebody keeps: 1080 lines on the shorter
    /// side, so 1920x1080 at 16:9 and 1080x1920 at 9:16.
    Delivery,
    /// A preview an assistant reads: 1280x720's pixels, in the frame's shape.
    /// The 2026-10-05 9:16 video asked for 23 stills at 1080x1920, and those
    /// pictures were most of what was re-read on every later call (#827).
    Preview,
    /// One cell of a contact sheet: a quarter of [`Budget::Preview`], so five
    /// together cost about one frame and a title in one still reads.
    Cell,
}

impl Budget {
    /// The pixels a [`Budget::Preview`] spends.
    pub const PREVIEW: u64 = 1280 * 720;
    /// The pixels a [`Budget::Cell`] spends.
    pub const CELL: u64 = 640 * 360;
}

impl Resolution {
    /// The raster of shape `aspect` that `budget` buys, both sides even.
    pub fn framed(aspect: Ratio, budget: Budget) -> Self {
        let (width, height) = match budget {
            Budget::Delivery => aspect.delivery(),
            Budget::Preview => aspect.spend(Budget::PREVIEW),
            Budget::Cell => aspect.spend(Budget::CELL),
        };
        // `Ratio` keeps both sides within 4:1 and `even` never goes below two.
        Self::new(width, height).expect("a framed raster is even and non-zero")
    }

    /// `project`'s frame at `budget`: what every default size is.
    pub fn of_project(project: &Project, budget: Budget) -> Self {
        Self::framed(project.aspect, budget)
    }
}

/// A size as a caller asks for one: an exact raster, `720x1280`, or a shape,
/// `9:16`, spent at whatever budget the asking surface draws at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sizing {
    /// Exactly these pixels.
    Exact(Resolution),
    /// This shape, at the surface's budget.
    Shaped(Ratio),
}

impl Sizing {
    /// The raster this asks for at `budget`, or `project`'s frame at it when
    /// nothing was asked.
    pub fn resolve(asked: Option<Self>, project: &Project, budget: Budget) -> Resolution {
        Self::shaped(asked, project.aspect, budget)
    }

    /// The raster this asks for at `budget`, or a frame of shape `aspect` at
    /// it when nothing was asked — for a caller holding the shape but not
    /// the project.
    pub fn shaped(asked: Option<Self>, aspect: Ratio, budget: Budget) -> Resolution {
        match asked {
            Some(Self::Exact(resolution)) => resolution,
            Some(Self::Shaped(aspect)) => Resolution::framed(aspect, budget),
            None => Resolution::framed(aspect, budget),
        }
    }
}

impl fmt::Display for Sizing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(resolution) => resolution.fmt(f),
            Self::Shaped(aspect) => aspect.fmt(f),
        }
    }
}

impl FromStr for Sizing {
    type Err = SizingError;

    /// Parses `1920x1080` or `9:16`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.contains(':') {
            return text
                .parse()
                .map(Self::Shaped)
                .map_err(|problem| SizingError(problem.to_string()));
        }
        text.parse()
            .map(Self::Exact)
            .map_err(|problem: ResolutionError| SizingError(problem.to_string()))
    }
}

/// A size that is neither a raster nor a shape, worded for whoever typed it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} — write a raster like 1080x1920, or a shape like 9:16")]
pub struct SizingError(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shape_is_spent_at_the_budget_and_a_raster_is_kept() {
        let project = Project::new("tall", scorsese_core::Fps::THIRTY);
        let tall: Sizing = "9:16".parse().expect("a shape");
        assert_eq!(
            Sizing::resolve(Some(tall), &project, Budget::Delivery).to_string(),
            "1080x1920"
        );
        assert_eq!(
            Sizing::resolve(Some(tall), &project, Budget::Preview).to_string(),
            "720x1280"
        );
        assert_eq!(
            Sizing::resolve(Some(tall), &project, Budget::Cell).to_string(),
            "360x640"
        );
        let exact: Sizing = "800x600".parse().expect("a raster");
        assert_eq!(
            Sizing::resolve(Some(exact), &project, Budget::Cell).to_string(),
            "800x600"
        );
        assert_eq!(
            Sizing::resolve(None, &project, Budget::Delivery),
            Resolution::HD
        );
        assert!("wide".parse::<Sizing>().is_err());
        assert!("8:1".parse::<Sizing>().is_err());
    }
}
