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
//! fitting is done by [`scorsese_compositor::Resample`].
//!
//! Done where is the other half (#880). The decoder only reads and numbers
//! each frame ([`Resized`]); a compositing worker fits it ([`Fitter`]), so the
//! resample runs on every worker at once instead of on the one thread that
//! reads the pipes in order. The output is the same bytes either way: the
//! arithmetic did not change, only the thread it runs on.

use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use scorsese_compositor::{CpuCompositor, Frame, Resample, Resolution};
use scorsese_core::probe::ProbeMedia;

use super::decode::{Fitting, Source};
use crate::error::{RenderError, Stage};
use crate::probe::Ffprobe;
use crate::raster::{cropped, fitted_inside};
use crate::tools::Tools;

/// The decode side of a resized source: what it reads frames into and how it
/// tells one picture from the next.
///
/// The resample itself is not done here. The decoder runs on the one thread
/// that reads every pipe in order, and a resample there is serial work the
/// compositing workers sit idle through — 26 ms a frame for 960×540 grown to
/// 1080p, 71 ms for 4K shrunk to it (#880). So a frame leaves this at the
/// source's own size, in a [`Pending`], and a worker fits it with the shared
/// [`Fitter`].
pub(super) struct Resized {
    fitter: Arc<Fitter>,
    /// The picture last read, and which one it was. A held still decodes the
    /// same bytes every frame, and resampling it again would be the whole
    /// cost of this path paid for nothing: a byte comparison here is far
    /// cheaper than the filter, and it is what lets a worker reuse the fit.
    previous: Option<(u64, Frame)>,
}

/// A frame read at its source's own size and not yet fitted, carried by a
/// job from the decoder to whichever worker composites it.
#[derive(Default)]
pub(crate) struct Pending {
    frame: Option<Frame>,
    /// Which picture it is, or `None` when there is nothing to fit — an
    /// opaque or native source, which was read straight into the layer's
    /// buffer, or one that ran out.
    picture: Option<u64>,
}

impl Pending {
    /// Nothing for a worker to fit.
    pub(super) const fn clear(&mut self) {
        self.picture = None;
    }
}

/// How one source's frames are fitted, shared by every worker compositing
/// them: the resample's weights, read-only, and the newest picture fitted, so
/// a held still is resampled once rather than once a frame.
pub(crate) struct Fitter {
    resample: Resample,
    last: Mutex<Option<(u64, Frame)>>,
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
            fitter: Arc::new(Fitter {
                resample: Resample::new(own, scaled, window, offset),
                last: Mutex::new(None),
            }),
            previous: None,
        }))
    }

    /// What a worker fits this source's frames with.
    pub(super) fn fitter(&self) -> Arc<Fitter> {
        Arc::clone(&self.fitter)
    }

    /// Reads the next frame from `pipe` into `pending`, at the source's own
    /// size, and numbers the picture: the same bytes as the last frame keep
    /// its number. `false` means the pipe ran out, which leaves nothing to fit.
    pub(super) fn read(
        &mut self,
        pipe: &mut impl Read,
        pending: &mut Pending,
    ) -> Result<bool, RenderError> {
        pending.picture = None;
        let own = self.fitter.resample.source();
        let frame = match &mut pending.frame {
            Some(frame) if frame.resolution() == own => frame,
            slot => slot.insert(Frame::black(own)),
        };
        if !super::decode::read_frame(pipe, frame)? {
            return Ok(false);
        }
        let picture = match &mut self.previous {
            Some((picture, before)) if before == frame => *picture,
            Some((picture, before)) => {
                before.bytes_mut().copy_from_slice(frame.bytes());
                *picture += 1;
                *picture
            }
            None => {
                self.previous = Some((0, frame.clone()));
                0
            }
        };
        pending.picture = Some(picture);
        Ok(true)
    }
}

impl Fitter {
    /// How big a frame of this source is as decoded, before it is fitted.
    pub(crate) fn decoded_bytes(&self) -> usize {
        let own = self.resample.source();
        own.width() as usize * own.height() as usize * scorsese_compositor::BYTES_PER_PIXEL
    }

    /// Fits `pending` into `into`, through `compositor`'s scratch. A picture
    /// already fitted — by this worker or another — is copied rather than
    /// resampled again; nothing to fit leaves `into` as it is.
    pub(crate) fn fit(&self, pending: &Pending, into: &mut Frame, compositor: &mut CpuCompositor) {
        let (Some(picture), Some(frame)) = (pending.picture, &pending.frame) else {
            return;
        };
        if let Some((fitted, last)) = &*self.lock()
            && *fitted == picture
        {
            into.bytes_mut().copy_from_slice(last.bytes());
            return;
        }
        compositor.resample(&self.resample, frame, into);
        // Kept only if nothing newer is: workers finish out of order, and the
        // newest picture is the one the frames still to come may repeat.
        let mut last = self.lock();
        match &mut *last {
            Some((fitted, _)) if *fitted >= picture => {}
            Some((fitted, kept)) => {
                *fitted = picture;
                kept.bytes_mut().copy_from_slice(into.bytes());
            }
            None => *last = Some((picture, into.clone())),
        }
    }

    /// The last picture fitted. A poisoned lock is a worker that panicked
    /// mid-copy, and the render is already failing; what it left is only ever
    /// a cache, so the next fit overwrites it.
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<(u64, Frame)>> {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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

    /// A 2×2 source grown to 4×4, read from `frames` laid end to end.
    fn growing(frames: &[[u8; 16]]) -> (Resized, std::io::Cursor<Vec<u8>>) {
        let resized = Resized {
            fitter: Arc::new(Fitter {
                resample: Resample::new(size(2, 2), size(4, 4), size(4, 4), (0, 0)),
                last: Mutex::new(None),
            }),
            previous: None,
        };
        (resized, std::io::Cursor::new(frames.concat()))
    }

    /// The same bytes are the same picture, and only a change moves the
    /// number on — which is all that lets a worker reuse a held still's fit.
    #[test]
    fn a_picture_is_numbered_by_whether_its_bytes_changed() {
        let (lit, dark) = ([200; 16], [9; 16]);
        let (mut resized, mut pipe) = growing(&[lit, lit, dark, lit]);
        let mut pending = Pending::default();
        let mut numbers = Vec::new();
        while resized
            .read(&mut pipe, &mut pending)
            .expect("a pipe in memory")
        {
            numbers.push(pending.picture);
        }
        assert_eq!(numbers, [Some(0), Some(0), Some(1), Some(2)]);
        assert_eq!(
            pending.picture, None,
            "a pipe that ran out leaves nothing to fit"
        );
    }

    /// A picture already fitted is copied rather than resampled: handed the
    /// same number over different bytes, the fit is the first one's. And a
    /// worker finishing an older picture late does not displace a newer one.
    #[test]
    fn a_picture_fitted_once_is_reused_and_never_by_an_older_one() {
        let (mut resized, mut pipe) = growing(&[[200; 16], [9; 16]]);
        let fitter = resized.fitter();
        let mut compositor = CpuCompositor::new();
        let (mut first, mut second) = (Pending::default(), Pending::default());
        resized
            .read(&mut pipe, &mut first)
            .expect("a pipe in memory");
        resized
            .read(&mut pipe, &mut second)
            .expect("a pipe in memory");
        let (mut older, mut newer) = (Frame::black(size(4, 4)), Frame::black(size(4, 4)));
        fitter.fit(&second, &mut newer, &mut compositor);
        fitter.fit(&first, &mut older, &mut compositor);
        assert_ne!(older, newer);

        second.frame = first.frame.clone();
        let mut again = Frame::black(size(4, 4));
        fitter.fit(&second, &mut again, &mut compositor);
        assert_eq!(again, newer, "picture 1 is still the one kept");
    }

    /// Nothing to fit — an opaque source read straight into its buffer, or
    /// one that ran out and was blanked — leaves the buffer as it was.
    #[test]
    fn nothing_to_fit_leaves_the_buffer_alone() {
        let (resized, _) = growing(&[]);
        let mut buffer = Frame::black(size(4, 4));
        buffer.fill_transparent();
        let before = buffer.clone();
        let pending = Pending::default();
        resized
            .fitter()
            .fit(&pending, &mut buffer, &mut CpuCompositor::new());
        assert_eq!(buffer, before);
    }
}
