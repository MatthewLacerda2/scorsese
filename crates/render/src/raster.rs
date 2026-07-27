//! What size a source's frames arrive at.
//!
//! Two of the three fit modes answer this from the render settings alone — a
//! fitted or filled source *becomes* the raster. `native` is the one that has
//! to ask the media how big it is, and this is where that asking is confined.
//!
//! The assets table is meant to already hold the answer, so the probed
//! metadata is used whenever it is there and a probe happens only when it is
//! not. That keeps the common case free of a subprocess without letting an
//! unprobed asset silently render at the wrong size.

use std::collections::HashMap;
use std::path::Path;

use scorsese_compositor::Resolution;
use scorsese_core::{Asset, AssetId, Fit, ProbeMedia};

use crate::error::RenderError;
use crate::pipe::Fitting;
use crate::plan::{Plan, Shot, Showing};
use crate::probe::Ffprobe;
use crate::tools::Tools;

/// The pixel size of every source a render needs one for, worked out once
/// before any frame is decoded.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sizes {
    native: HashMap<AssetId, Resolution>,
}

impl Sizes {
    /// Measures every asset a clip in this plan wants at its native size.
    ///
    /// Up front rather than per segment, because a clip cut across three
    /// segments by boundaries on other tracks is one source, and asking ffprobe
    /// three times about the same file is three processes for one fact.
    pub fn measure(
        tools: &Tools,
        plan: &Plan<'_>,
        project_root: &Path,
    ) -> Result<Self, RenderError> {
        let mut native: HashMap<AssetId, Resolution> = HashMap::new();
        let probe = Ffprobe::new(tools.clone());

        for shot in plan.segments().iter().flat_map(|segment| &segment.layers) {
            // A drawn asset has no native size to measure: text and slug cards
            // are rasterised at whatever the render's raster is, so `fit` says
            // nothing about them and there is no file to ask.
            if shot.clip.fit != Fit::Native
                || !shot.asset.kind.is_file_backed()
                || shot.showing != Showing::Media
                || native.contains_key(&shot.asset.id)
            {
                continue;
            }
            // A generated file that has gone missing shows a card too, and
            // probing it would be a subprocess spent to fail. Asked of the
            // filesystem rather than the document, which is the only place the
            // answer is.
            if matches!(
                crate::slug::standing(shot, project_root)?,
                crate::slug::Standing::Card(_)
            ) {
                continue;
            }
            let size = measure_one(&probe, shot.asset, project_root).map_err(|reason| {
                RenderError::UnknownSourceSize {
                    clip: shot.clip.id.to_string(),
                    asset: shot.asset.id.to_string(),
                    reason,
                }
            })?;
            native.insert(shot.asset.id.clone(), size);
        }
        Ok(Self { native })
    }

    /// How this shot meets the raster.
    ///
    /// A native shot whose size was never measured cannot happen — every one in
    /// the plan is measured — and falling back to fitting it is the safe way to
    /// be wrong: the picture is the wrong size, rather than the pipe being read
    /// at the wrong stride and every later frame sliding.
    pub fn fitting(&self, shot: &Shot<'_>) -> Fitting {
        match shot.clip.fit {
            Fit::Fit => Fitting::Fit,
            Fit::Fill => Fitting::Fill,
            Fit::Native => self
                .native
                .get(&shot.asset.id)
                .map_or(Fitting::Fit, |&size| Fitting::Native(size)),
        }
    }
}

/// The asset's own pixel size, from what was probed at import or from a probe
/// run now. The `Err` is why it could not be had, in words a report can carry.
fn measure_one(probe: &Ffprobe, asset: &Asset, project_root: &Path) -> Result<Resolution, String> {
    if let Some(size) = asset.media.and_then(dimensions) {
        return Ok(size);
    }
    let path = asset
        .path
        .as_ref()
        .ok_or_else(|| "it has no media file".to_owned())?;
    let media = probe
        .probe(&path.resolve(project_root))
        .map_err(|error| error.message)?;
    dimensions(media).ok_or_else(|| "the file reports no picture size".to_owned())
}

/// A metadata row's width and height as a raster, when it has both. The even
/// rule does not apply: this is a source, not something being encoded.
fn dimensions(media: scorsese_core::MediaMetadata) -> Option<Resolution> {
    let (width, height) = (media.width?, media.height?);
    Resolution::source(width, height).ok()
}
