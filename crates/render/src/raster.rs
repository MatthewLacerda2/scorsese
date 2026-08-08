//! What size a source's frames arrive at.
//!
//! One of the three fit modes answers this from the render settings alone: a
//! *filled* source becomes the raster, because covering it is what filling
//! means. The other two have to ask the media how big it is, and this is where
//! that asking is confined. `native` needs it to leave the source alone;
//! `fit` needs it to know the rectangle the picture ends up occupying, which
//! is smaller than the raster whenever the aspects differ and is the whole
//! reason an anchor has anything to rest against.
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
pub(crate) struct Sizes {
    measured: HashMap<AssetId, Resolution>,
}

impl Sizes {
    /// Measures every asset a clip in this plan needs a source size for, which
    /// is every `native` one and every `fit` one.
    ///
    /// Up front rather than per segment, because a clip cut across three
    /// segments by boundaries on other tracks is one source, and asking ffprobe
    /// three times about the same file is three processes for one fact.
    pub(crate) fn measure(
        tools: &Tools,
        plan: &Plan<'_>,
        project_root: &Path,
    ) -> Result<Self, RenderError> {
        let mut measured: HashMap<AssetId, Resolution> = HashMap::new();
        let probe = Ffprobe::new(tools.clone());

        for shot in plan.segments().iter().flat_map(|segment| &segment.layers) {
            // A drawn asset has no size of its own to measure: text and slug
            // cards are rasterised at whatever the render's raster is, so `fit`
            // says nothing about them and there is no file to ask.
            if !matches!(shot.clip.fit, Fit::Native | Fit::Fit)
                || !shot.asset.kind.is_file_backed()
                || shot.showing != Showing::Media
                || measured.contains_key(&shot.asset.id)
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
            measured.insert(shot.asset.id.clone(), size);
        }
        Ok(Self { measured })
    }

    /// How this shot meets the raster.
    ///
    /// A shot whose size was never measured cannot happen — every one in the
    /// plan that needs one is measured — and the fallbacks are the safe way to
    /// be wrong. A native layer falls back to being fitted and a fitted one to
    /// being padded by ffmpeg: in both, the picture is somewhere unintended,
    /// rather than the pipe being read at the wrong stride and every later
    /// frame sliding.
    ///
    /// A **cropped** layer is the cropped pixels, for `native` and `fit`
    /// alike: the crop runs before the fit, so by the time either is deciding
    /// what arrives, the source is already the rectangle rather than the file.
    /// Getting this wrong would not merely be the wrong size on screen — the
    /// buffer reading the pipe is sized from this, so it would be read at the
    /// wrong stride and every later frame would slide.
    pub(crate) fn fitting(&self, shot: &Shot<'_>, raster: Resolution) -> Fitting {
        let source = || {
            self.measured
                .get(&shot.asset.id)
                .map(|&size| cropped(size, shot.clip.crop))
        };
        match shot.clip.fit {
            Fit::Fit => source().map_or(Fitting::FitPadded, |size| {
                Fitting::Fit(fitted_inside(size, raster))
            }),
            Fit::Fill => Fitting::Fill,
            Fit::Native => source().map_or(Fitting::FitPadded, Fitting::Native),
        }
    }
}

/// The largest rectangle with the source's aspect that fits inside the raster.
///
/// This is what `scale=…:force_original_aspect_ratio=decrease` computes, worked
/// out here instead so the number in the filter and the number the read buffer
/// is sized from are the same number rather than two that agree by inspection.
///
/// A source already inside the raster is still scaled **up** to meet an edge,
/// which is what fitting has always done and is not the same question as
/// whether it should — `native` is the fit mode for leaving a source alone.
///
/// Both axes are kept at a pixel or more. A source so much wider than the
/// raster that its height rounds to nothing is a picture nothing can be read
/// into, and a one-pixel line is a better answer than a failed render.
fn fitted_inside(source: Resolution, raster: Resolution) -> Resolution {
    let scale = f64::from(raster.width()) / f64::from(source.width());
    let scale = scale.min(f64::from(raster.height()) / f64::from(source.height()));
    let width = (f64::from(source.width()) * scale).round() as u32;
    let height = (f64::from(source.height()) * scale).round() as u32;
    Resolution::source(
        width.clamp(1, raster.width()),
        height.clamp(1, raster.height()),
    )
    .unwrap_or(raster)
}

/// What a crop leaves of a source, in pixels.
///
/// ffmpeg's `crop` rounds the same way — `iw*w` truncated to whole pixels — and
/// the two must agree, because one sizes the buffer and the other fills it.
/// At least one pixel each way: a rectangle that rounds to nothing is a raster
/// nothing can be read into, and validation has already established that the
/// fractions themselves enclose some of the source.
fn cropped(size: Resolution, crop: Option<scorsese_core::Crop>) -> Resolution {
    let Some(crop) = crop else {
        return size;
    };
    let width = (f64::from(size.width()) * crop.width) as u32;
    let height = (f64::from(size.height()) * crop.height) as u32;
    Resolution::source(width.max(1), height.max(1)).unwrap_or(size)
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
