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
        Self::build(plan, project_root, Some(&Ffprobe::new(tools.clone())))
    }

    /// Only the sizes the document already records — no subprocess, and a
    /// source it says nothing about is simply absent.
    ///
    /// What a **query** about a layout can afford. Measuring costs an ffprobe
    /// per unprobed file, which is a price worth paying to produce a video and
    /// not to answer a question about one; [`Sizes::is_unmeasured`] is how the
    /// caller says so out loud rather than quietly answering with the wrong
    /// rectangle.
    pub(crate) fn recorded(plan: &Plan<'_>, project_root: &Path) -> Self {
        Self::build(plan, project_root, None).unwrap_or_default()
    }

    /// True when this shot's own size decides where its picture lands and
    /// nothing established it — so any rectangle claimed for it would be a
    /// guess.
    pub(crate) fn is_unmeasured(&self, shot: &Shot<'_>) -> bool {
        matches!(shot.clip.fit, Fit::Native | Fit::Fit)
            && !self.measured.contains_key(&shot.asset.id)
    }

    /// Every source size this plan needs: from the document, and — when there
    /// is a `probe` to ask — from the file itself when the document is silent.
    ///
    /// Without one, a size nothing recorded is left out rather than refused.
    /// Nothing is being rendered, so being unable to answer about one clip is
    /// an answer about that clip and not a failure of the whole call.
    fn build(
        plan: &Plan<'_>,
        project_root: &Path,
        probe: Option<&Ffprobe>,
    ) -> Result<Self, RenderError> {
        let mut measured: HashMap<AssetId, Resolution> = HashMap::new();

        for shot in plan.segments().iter().flat_map(|segment| segment.shots()) {
            // A drawn asset has no size of its own to measure: text and slug
            // cards are rasterised at whatever the render's raster is, so `fit`
            // says nothing about them and there is no file to ask.
            if !matches!(shot.clip.fit, Fit::Native | Fit::Fit)
                || !shot.picture().kind.is_file_backed()
                || shot.showing != Showing::Media
                || measured.contains_key(&shot.asset.id)
            {
                continue;
            }
            // A generated file that has gone missing shows a card too, and
            // probing it would be a subprocess spent to fail. Asked of the
            // filesystem rather than the document, which is the only place the
            // answer is.
            match crate::slug::standing(shot, project_root) {
                Ok(crate::slug::Standing::Card(_)) => continue,
                Ok(crate::slug::Standing::Media(_)) => {}
                // A file that is not there stops a render. It does not stop a
                // question about one: the caller reports that clip and answers
                // about the rest of the frame.
                Err(_) if probe.is_none() => continue,
                Err(error) => return Err(error),
            }
            let Some(probe) = probe else {
                measured.extend(
                    dimensions_of(shot.picture()).map(|size| (shot.asset.id.clone(), size)),
                );
                continue;
            };
            // A sequence is measured by its first still, and keyed by its own
            // id: every still is one size, and the clip shows the sequence.
            let size = measure_one(probe, shot.picture(), project_root).map_err(|reason| {
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
pub(crate) fn fitted_inside(source: Resolution, raster: Resolution) -> Resolution {
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
/// Each kept side is `round(size * fraction)`, held between one pixel and the
/// whole side: a rectangle that rounds to nothing is a raster nothing can be
/// read into, and validation has already established that the fractions
/// themselves enclose some of the source.
///
/// This sizes the buffer the pipe is read into and ffmpeg's `crop` fills it,
/// so the two must agree to the pixel, and they once did not: this truncated
/// while ffmpeg rounded, and rounded the width down to even on 4:2:0 input, so
/// a native or alpha source read at the wrong stride and slid (#1089). Now
/// [`crop_filter`] hands ffmpeg this same arithmetic as its own expression,
/// with `exact=1` so it rounds nothing further, and the two agree by
/// construction.
pub(crate) fn cropped(size: Resolution, crop: Option<scorsese_core::Crop>) -> Resolution {
    let Some(crop) = crop else {
        return size;
    };
    let kept = |side: u32, fraction: f64| {
        let pixels = (f64::from(side) * fraction).round();
        // In range by the clamp, so the cast neither truncates nor wraps.
        pixels.clamp(1.0, f64::from(side)) as u32
    };
    Resolution::source(
        kept(size.width(), crop.width),
        kept(size.height(), crop.height),
    )
    .unwrap_or(size)
}

/// ffmpeg's `crop` filter for `crop`, keeping exactly the pixels [`cropped`]
/// counts.
///
/// Written in terms of the input's own `iw`/`ih` rather than as numbers baked
/// from a measured size, because the file decoded is not always the one that
/// was measured: a preview reads a smaller proxy, and an asset can be replaced
/// by a bigger capture of the same thing. Rounding is to nearest, the same
/// `round` in the same double arithmetic as [`cropped`]; `exact=1` stops
/// ffmpeg rounding a subsampled source's rectangle to even on top of that,
/// which on 4:2:0 input it would otherwise do. The offset needs no clamp of
/// ours: ffmpeg keeps the rectangle inside the frame, and where it sits never
/// sizes a buffer.
pub(crate) fn crop_filter(crop: scorsese_core::Crop) -> String {
    format!(
        "crop=w='clip(round(iw*{w}),1,iw)':h='clip(round(ih*{h}),1,ih)':\
         x='round(iw*{x})':y='round(ih*{y})':exact=1",
        w = crop.width,
        h = crop.height,
        x = crop.x,
        y = crop.y,
    )
}

/// The asset's own pixel size, from what was probed at import or from a probe
/// run now. The `Err` is why it could not be had, in words a report can carry.
fn measure_one(probe: &Ffprobe, asset: &Asset, project_root: &Path) -> Result<Resolution, String> {
    if let Some(size) = dimensions_of(asset) {
        return Ok(size);
    }
    let path = asset
        .path
        .as_ref()
        .ok_or_else(|| "it has no media file".to_owned())?;
    let file = path
        .resolve(project_root)
        .map_err(|refused| refused.to_string())?;
    let media = probe.probe(&file).map_err(|error| error.message)?;
    dimensions(media).ok_or_else(|| "the file reports no picture size".to_owned())
}

/// The size an asset's probed metadata already records, if it records one —
/// the answer a measurement gets for free, and the only one a query takes.
fn dimensions_of(asset: &Asset) -> Option<Resolution> {
    asset.media.and_then(dimensions)
}

/// A metadata row's width and height as a raster, when it has both. The even
/// rule does not apply: this is a source, not something being encoded.
fn dimensions(media: scorsese_core::MediaMetadata) -> Option<Resolution> {
    let (width, height) = (media.width?, media.height?);
    Resolution::source(width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::Crop;

    fn third() -> Option<Crop> {
        Some(Crop {
            x: 1.0 / 3.0,
            y: 1.0 / 3.0,
            width: 1.0 / 3.0,
            height: 1.0 / 3.0,
        })
    }

    /// Rounded to nearest, as ffmpeg's expression in [`crop_filter`] rounds:
    /// a third of 50 is 16.67 and keeps 17, of 52 is 17.33 and keeps 17 —
    /// odd, which `exact=1` lets a 4:2:0 source keep too.
    #[test]
    fn a_crop_keeps_the_nearest_whole_pixels() {
        let size = |width, height| Resolution::source(width, height).expect("a size");
        assert_eq!(cropped(size(50, 52), third()), size(17, 17));
        assert_eq!(cropped(size(1920, 1080), third()), size(640, 360));
    }

    /// Never nothing, never more than the source.
    #[test]
    fn a_crop_keeps_at_least_a_pixel_and_at_most_the_source() {
        let size = Resolution::source(10, 10).expect("a size");
        let sliver = Crop {
            width: 0.01,
            height: 1.0,
            ..Crop::default()
        };
        assert_eq!(
            cropped(size, Some(sliver)),
            Resolution::source(1, 10).expect("a size")
        );
    }

    #[test]
    fn the_filter_rounds_the_same_way_and_exactly() {
        let filter = crop_filter(third().expect("a crop"));
        assert!(filter.contains("w='clip(round(iw*0.3333333333333333),1,iw)'"));
        assert!(filter.ends_with(":exact=1"));
    }
}
