//! Which raster `still` composites at: a size the caller named, or a preview
//! of the project's own frame (#827, #1278).
//!
//! The default used to be one landscape raster, and a vertical edit had to pass
//! `resolution` to see its own shape — and the one portrait size a model has to
//! hand is the delivery's. The 2026-10-05 9:16 video asked for every one of its
//! 23 stills at 1080x1920, more than twice the pixels of the default, and those
//! pictures were most of what the assistant re-read on every later call. So the
//! preview is a **pixel budget** ([`Budget::Preview`], [`Budget::Cell`]) spent
//! in the project's shape, or in the shape the caller names (`"9:16"`).

use scorsese_core::Project;
use scorsese_render::{Budget, Resolution, Sizing};

/// The raster to composite at, and — when the project chose its shape — the
/// words saying so, for the reply's first sentence.
pub(super) fn choose(
    asked: Option<&str>,
    project: &Project,
    budget: Budget,
) -> Result<(Resolution, String), String> {
    let asked = asked
        .map(str::parse::<Sizing>)
        .transpose()
        .map_err(|problem| format!("resolution: {problem}"))?;
    let shaped = match asked {
        None => format!(" (the project's {} frame)", project.aspect),
        Some(_) => String::new(),
    };
    Ok((Sizing::resolve(asked, project, budget), shaped))
}

#[cfg(test)]
mod tests;
