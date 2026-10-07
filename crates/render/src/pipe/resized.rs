//! A source with alpha, brought to the size its fitting asks for by the
//! compositor instead of by ffmpeg (#868).
//!
//! An opaque source is still scaled inside ffmpeg's filter chain: swscale on
//! three channels is plain arithmetic every version agrees on closely enough
//! for the golden tolerance. A transparent one needed `premultiply` around the
//! `scale`, and that filter's rounding changed in FFmpeg 8 — enough to move
//! three page goldens on Arch while CI's 6.1 passed them. So for a source with
//! alpha the decoder asks ffmpeg for nothing but `format=rgba` at the source's
//! own (cropped) size, which every version decodes to the same bytes, and the
//! fitting is done here by [`scorsese_compositor::Resample`].

use std::path::Path;

use scorsese_compositor::{Frame, Resample, Resolution};
use scorsese_core::probe::ProbeMedia;

use super::decode::{Fitting, Source};
use crate::error::{RenderError, Stage};
use crate::probe::Ffprobe;
use crate::raster::{cropped, fitted_inside};
use crate::tools::Tools;

/// A source's frames as decoded, and the resample that fits each one.
pub(super) struct Resized {
    resample: Resample,
    /// The frame just read from the pipe, at the source's own size.
    decoded: Frame,
    /// The one before it, and what it resampled to. A held still decodes
    /// the same picture every frame, and resampling it again would be the
    /// whole cost of this path paid for nothing: a byte comparison is far
    /// cheaper than the filter.
    previous: Option<(Frame, Frame)>,
}

impl Resized {
    /// The resample `source` needs, or `None` when it needs none: a source
    /// without alpha, one left at its own size, or one already the size its
    /// fitting asks for — a page captured at the raster's own size, which is
    /// then decoded and shown untouched.
    pub(super) fn plan(
        tools: &Tools,
        source: &Source,
        raster: Resolution,
    ) -> Result<Option<Self>, RenderError> {
        if !source.has_alpha || matches!(source.fitting, Fitting::Native(_)) {
            return Ok(None);
        }
        let file = source.listed.first().unwrap_or(&source.file);
        let own = cropped(measure(tools, file)?, source.crop);
        let (scaled, window, offset) = geometry(source.fitting, own, raster);
        if scaled == own && window == own && offset == (0, 0) {
            return Ok(None);
        }
        Ok(Some(Self {
            resample: Resample::new(own, scaled, window, offset),
            decoded: Frame::black(own),
            previous: None,
        }))
    }

    /// The size the pipe hands over, which is what the read buffer must be.
    pub(super) fn decoded(&mut self) -> &mut Frame {
        &mut self.decoded
    }

    /// Fits the frame just read into `into`.
    pub(super) fn fit(&mut self, into: &mut Frame) {
        let fresh = self
            .previous
            .as_ref()
            .is_none_or(|(before, _)| *before != self.decoded);
        if fresh {
            let (source, window) = (self.resample.source(), self.resample.window());
            let (before, fitted) = self
                .previous
                .get_or_insert_with(|| (Frame::black(source), Frame::black(window)));
            std::mem::swap(before, &mut self.decoded);
            self.resample.apply(before, fitted);
        }
        let (_, fitted) = self.previous.as_ref().expect("filled just above");
        into.bytes_mut().copy_from_slice(fitted.bytes());
    }
}

/// What the scale is, how much of it is kept, and where that starts, for each
/// fitting — the same three numbers ffmpeg's `scale` and `crop`/`pad` used to
/// work out, decided here so the buffer and the picture cannot disagree.
fn geometry(
    fitting: Fitting,
    own: Resolution,
    raster: Resolution,
) -> (Resolution, Resolution, (i64, i64)) {
    let centred = |outer: u32, inner: u32| (i64::from(outer) - i64::from(inner)) / 2;
    match fitting {
        Fitting::Fit(fitted) => (fitted, fitted, (0, 0)),
        Fitting::FitPadded => {
            let fitted = fitted_inside(own, raster);
            let offset = (
                -centred(raster.width(), fitted.width()),
                -centred(raster.height(), fitted.height()),
            );
            (fitted, raster, offset)
        }
        Fitting::Fill => {
            let covering = covering(own, raster);
            let offset = (
                centred(covering.width(), raster.width()),
                centred(covering.height(), raster.height()),
            );
            (covering, raster, offset)
        }
        Fitting::Native(size) => (size, size, (0, 0)),
    }
}

/// The smallest rectangle with the source's aspect that covers the raster —
/// `force_original_aspect_ratio=increase`, rounded, and never short of the
/// raster on either axis.
fn covering(own: Resolution, raster: Resolution) -> Resolution {
    let scale = f64::from(raster.width()) / f64::from(own.width());
    let scale = scale.max(f64::from(raster.height()) / f64::from(own.height()));
    let width = ((f64::from(own.width()) * scale).round() as u32).max(raster.width());
    let height = ((f64::from(own.height()) * scale).round() as u32).max(raster.height());
    Resolution::source(width, height).unwrap_or(raster)
}

/// The file's own picture size, asked of ffprobe.
fn measure(tools: &Tools, file: &Path) -> Result<Resolution, RenderError> {
    let failed = |message: String| RenderError::Ffmpeg {
        stage: Stage::Decode,
        subject: file.display().to_string(),
        message,
    };
    let media = Ffprobe::new(tools.clone())
        .probe(file)
        .map_err(|error| failed(error.message))?;
    let (Some(width), Some(height)) = (media.width, media.height) else {
        return Err(failed("the file reports no picture size".to_owned()));
    };
    Resolution::source(width, height).map_err(|error| failed(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: u32, height: u32) -> Resolution {
        Resolution::source(width, height).expect("a legal size")
    }

    /// A page captured at six times the raster fills it exactly, from the
    /// corner: the aspect already matches, so nothing is cut off.
    #[test]
    fn a_capture_of_the_same_shape_fills_the_raster_whole() {
        let fill = geometry(Fitting::Fill, size(960, 540), size(160, 90));
        assert_eq!(fill, (size(160, 90), size(160, 90), (0, 0)));
    }

    /// A square filling a wide raster is scaled to the raster's width and
    /// loses its top and bottom equally.
    #[test]
    fn a_fill_of_another_shape_keeps_the_middle() {
        let fill = geometry(Fitting::Fill, size(100, 100), size(160, 90));
        assert_eq!(fill, (size(160, 160), size(160, 90), (0, 35)));
    }

    /// A padded fit is the fitted picture in the middle of a raster-sized
    /// window, which the negative offset says.
    #[test]
    fn a_padded_fit_sits_in_the_middle_of_the_raster() {
        let padded = geometry(Fitting::FitPadded, size(100, 100), size(160, 90));
        assert_eq!(padded, (size(90, 90), size(160, 90), (-35, 0)));
    }
}
