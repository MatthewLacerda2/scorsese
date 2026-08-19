//! Getting raw frames out of a source file.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use crate::error::{RenderError, Stage};
use crate::settings::RenderSettings;
use crate::tools::Tools;
use scorsese_compositor::{Frame, PIXEL_FORMAT, Resolution};
use scorsese_core::{Crop, Speed};

/// How a source meets the render's raster, resolved to pixels.
///
/// [`scorsese_core::Fit`] says what the author asked for; this is what the
/// decoder does about it, which for two of the three means already knowing the
/// source's own size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fitting {
    /// Scale to fit inside the raster, and stop there. The resolution is the
    /// **fitted picture's own rectangle**, smaller than the raster on whichever
    /// axis letterboxes.
    ///
    /// Nothing is padded. The bars are the canvas showing through rather than
    /// transparent pixels belonging to the layer, which is what leaves an
    /// anchor a gap to rest the picture against — see `transform_of` in the
    /// compositor. Padding here would bake the gap into the layer's alpha,
    /// where nothing downstream can see it.
    Fit(Resolution),
    /// Scale to fit inside the raster and let ffmpeg pad the rest transparent.
    ///
    /// The one case the rectangle above cannot be worked out in: a source whose
    /// own size could not be had. The layer then arrives raster-sized and an
    /// anchor on it is a no-op, which is worse than [`Self::Fit`] and better
    /// than refusing to render — being wrong about where the picture rests
    /// beats not producing one.
    FitPadded,
    /// Scale to cover the raster; crop the overflow off the edges.
    Fill,
    /// Leave the source alone. It arrives at this — its own — size.
    Native(Resolution),
}

impl Fitting {
    /// The size frames come out at, and so the size of the buffer that reads
    /// them. Filling produces the render's raster by construction; fitting
    /// produces the picture's own rectangle, and native whatever the source
    /// happens to be.
    pub(crate) fn raster(self, settings: &RenderSettings) -> Resolution {
        self.raster_at(settings.resolution)
    }

    /// The same, against a raster rather than a whole set of settings — for a
    /// caller asking where a picture would land without rendering one, which
    /// has no settings to hand. Nothing about the size a source arrives at
    /// depends on any of the others.
    pub(crate) const fn raster_at(self, raster: Resolution) -> Resolution {
        match self {
            Self::FitPadded | Self::Fill => raster,
            Self::Fit(fitted) | Self::Native(fitted) => fitted,
        }
    }
}

/// What to decode, and how much of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Source {
    /// The media to read, already resolved against the project root — nothing
    /// below here knows what a project directory is.
    pub(crate) file: PathBuf,
    /// An image, which has no timeline of its own and is held for as long as
    /// the clip lasts.
    ///
    /// Held, not frozen: an image format that carries an animation — a gif,
    /// an avif — plays it and repeats it for the clip's length. Either way the
    /// length is the clip's to decide, which is why an image's own duration is
    /// never recorded against the asset.
    pub(crate) still: bool,
    /// Where to start in the source, in wall-clock seconds. Seconds because
    /// that is the unit ffmpeg seeks in; the conversion from the timeline grid
    /// happened before we got here.
    pub(crate) seek_seconds: f64,
    /// How fast to run the source against the output grid. [`Speed::NORMAL`]
    /// leaves the timing alone, and the filter that would express it is left
    /// out entirely rather than written as a no-op — a render of ordinary clips
    /// has to reach ffmpeg exactly as it did before there was a rate to choose.
    pub(crate) speed: Speed,
    /// How many frames to ask for, on the output grid.
    pub(crate) frames: u64,
    /// How this source meets the raster.
    pub(crate) fitting: Fitting,
    /// Whether the file carries an alpha channel, as the document records it.
    /// An asset nobody has probed collapses to `false` here: the difference
    /// between "no alpha" and "nobody looked" matters where the fact is
    /// recorded, and this is the stage that has to act on one answer or the
    /// other. Acting as though there is none is what the decoder did before
    /// the fact existed.
    pub(crate) has_alpha: bool,
    /// Which rectangle of the source is shown. Absent means all of it.
    pub(crate) crop: Option<Crop>,
}

/// An ffmpeg process decoding one source into raw frames on its stdout.
///
/// One process per segment rather than one for the whole timeline: a segment
/// has one source, one seek point, and one frame count, and a process per
/// segment keeps that mapping obvious. Sequencing several sources into one
/// ffmpeg invocation would be handing our job — deciding what is on screen
/// when — back to ffmpeg.
pub(crate) struct Decoder {
    child: Child,
    stdout: ChildStdout,
    subject: String,
    raster: Resolution,
}

impl Decoder {
    /// Starts decoding. Frames arrive re-timed to the render's framerate and
    /// fitted the way the clip asked to be: fitting a source into the output
    /// raster is a decode concern, and doing it here means every frame reaching
    /// our process is already the size the compositor will place.
    ///
    /// Sources of a different shape are never stretched. `fit` letterboxes them
    /// — a vertical phone clip in a 16:9 render gets transparent at the sides —
    /// `fill` crops instead, and `native` leaves the source at its own size for
    /// the compositor to rest on the canvas.
    pub(crate) fn start(
        tools: &Tools,
        source: &Source,
        settings: &RenderSettings,
    ) -> Result<Self, RenderError> {
        let rate = format!("{}/{}", settings.fps.num(), settings.fps.den());
        let mut command = tools.ffmpeg();
        command.args(["-nostdin", "-v", "error"]);
        if source.still {
            hold(&mut command, &source.file, &rate);
        } else if source.seek_seconds > 0.0 {
            command
                .arg("-ss")
                .arg(format!("{:.6}", source.seek_seconds));
        }
        command
            .arg("-i")
            .arg(&source.file)
            .args(["-frames:v", &source.frames.to_string()])
            .arg("-vf")
            .arg(video_filter(settings, source))
            .args(["-an", "-pix_fmt", PIXEL_FORMAT, "-f", "rawvideo", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| RenderError::Spawn {
            stage: Stage::Decode,
            source,
        })?;
        let stdout = child
            .stdout
            .take()
            .expect("stdout was piped when the process was spawned");
        Ok(Self {
            child,
            stdout,
            subject: source.file.display().to_string(),
            raster: source.fitting.raster(settings),
        })
    }

    /// The size the frames it produces are. Asked of the decoder rather than
    /// worked out again by the caller, so a buffer can only ever be the size
    /// the pipe is about to fill — a mismatch would not fail, it would slide
    /// every later frame along by the difference.
    pub(crate) const fn raster(&self) -> Resolution {
        self.raster
    }

    /// Reads the next frame into `frame`. `false` means the source ran out —
    /// which is a fact about the media, not a failure: a clip longer than its
    /// source is a project mistake, and the caller reports it.
    pub(crate) fn read_into(&mut self, frame: &mut Frame) -> Result<bool, RenderError> {
        match self.stdout.read_exact(frame.bytes_mut()) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(false),
            Err(source) => Err(RenderError::Pipe {
                stage: Stage::Decode,
                source,
            }),
        }
    }

    /// Waits for ffmpeg and reports what it said if it failed.
    pub(crate) fn finish(mut self) -> Result<(), RenderError> {
        // Drain anything still in flight first. ffmpeg blocks writing into a
        // full pipe, so waiting on a process we stopped reading from would
        // deadlock rather than exit.
        std::io::copy(&mut self.stdout, &mut std::io::sink()).map_err(|source| {
            RenderError::Pipe {
                stage: Stage::Decode,
                source,
            }
        })?;
        super::finish(self.child, Stage::Decode, &self.subject)
    }
}

/// Extensions ffmpeg reads through its `image2` demuxer.
///
/// `-loop` is a **private option of that demuxer**, not a general one, so the
/// list is not decoration: passing it to any other demuxer is an error before
/// a frame is read. Held to what ffmpeg actually accepts by the test below it.
const IMAGE2: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp"];

/// Asks ffmpeg to hold a still for as long as the clip needs it.
///
/// There are two ways to say that and they are not interchangeable. `-loop 1`
/// is the `image2` demuxer repeating its one picture, and it is what every
/// still has always been decoded with. `-stream_loop -1` is the *container*
/// being played again from the top, which every demuxer understands.
///
/// Both are correct for a single picture and produce byte-identical frames, so
/// the choice is made on the two things that differ. `-loop 1` reaches a
/// demuxer that does not take it — gif and avif, whose formats have their own
/// notion of a frame sequence — as `Option loop not found`, which fails the
/// render before it decodes anything. And `-stream_loop` costs about half as
/// much again in decode time, because the file is opened and read once per
/// repeat rather than held; measured at 3.7s against 2.4s for 300 frames of a
/// 1920x1080 png.
///
/// So: the cheap mechanism wherever it works, and the general one everywhere
/// else. What the general one buys beyond merely working is that an **animated**
/// gif or avif plays rather than freezing on its first frame, and loops for as
/// long as the clip lasts — which is what a still of an animation should do,
/// and what makes the clip's length the author's decision rather than the
/// file's.
fn hold(command: &mut Command, file: &Path, rate: &str) {
    if reads_through_image2(file) {
        command.args(["-loop", "1", "-framerate", rate]);
    } else {
        command.args(["-stream_loop", "-1"]);
    }
}

/// Whether ffmpeg will open this file with the demuxer that takes `-loop`.
///
/// Decided from the extension, which is the same evidence ffmpeg itself uses
/// to pick a demuxer for these formats. Anything unrecognised takes the
/// general path: being slower is recoverable, and refusing to decode is not.
fn reads_through_image2(file: &Path) -> bool {
    file.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            let extension = extension.to_ascii_lowercase();
            IMAGE2.contains(&extension.as_str())
        })
}

/// Re-time first, then fit — in that order, because dropping frames before
/// scaling means not scaling the frames that get dropped.
///
/// **A clip's speed is the first thing in the chain**, ahead of the re-time. It
/// is `setpts`, which rewrites the source's own timestamps and nothing else:
/// at 2× every frame's presentation time is halved, so the `fps` filter behind
/// it sees a source running twice as fast and picks accordingly. That is the
/// whole of the frame puller's speed work, and it is deliberately expressed to
/// ffmpeg as *timing* rather than done by us picking frames — deciding what is
/// on screen when is ours, and which source frame is nearest a given instant is
/// the same conform question `fps` already answers everywhere else.
///
/// A clip at its normal rate gets no `setpts` at all, rather than one that
/// multiplies by one. The filter graph an ordinary render sends to ffmpeg has
/// to be the graph it sent before this field existed, or every reference frame
/// in the golden set is asserting something new.
///
/// `fit` scales the source to the largest rectangle that sits inside the raster
/// and **stops there**: the bars a letterbox leaves are the canvas showing
/// through, not pixels the layer owns. On the bottom layer that is
/// indistinguishable from padding transparently — the canvas underneath is
/// black anyway — but on an upper track it is the whole point twice over. A
/// narrow clip over a wide one shows the wide one at the sides, and an `anchor`
/// on the narrow one has a gap to rest it against, which it does not once the
/// gap has been baked into the layer's alpha.
///
/// [`Fitting::FitPadded`] is the older form of the same thing and reaches this
/// only when the source's size could not be measured, where `format=rgba`
/// before the `pad` is what keeps the bars transparent rather than black.
///
/// `fill` is the same scale with the rounding turned the other way, so the
/// source covers the raster instead of sitting inside it, and a centred crop
/// takes the overflow off. Nothing is padded because nothing is left over.
///
/// `native` scales nothing and pads nothing: the frames come out at the source's
/// own size, and where they sit on the canvas is the compositor's business.
///
/// A `crop` goes **first**, ahead of all three. The order is `source → crop →
/// fit into the raster`, and it is the only one that makes sense: cropping
/// after the fit would be cropping the *output*, which is a matte and a
/// different feature. So after a crop it is the **cropped** rectangle that
/// `fit`, `fill` and `native` reconcile against the raster — which means a crop
/// that changes the aspect changes what `fit` does. That is correct and it
/// should not surprise anyone, which is why it is written here.
fn video_filter(settings: &RenderSettings, source: &Source) -> String {
    let (fitting, crop) = (source.fitting, source.crop);
    let rate = format!("fps={}/{}", settings.fps.num(), settings.fps.den());
    // A still has no timestamps of its own to rewrite — it is one frame looped
    // at the output rate — so a speed on one is a rate with nothing to apply to
    // rather than an error. Held is held.
    let rate = if source.still || source.speed.is_normal() {
        rate
    } else {
        format!("setpts=PTS/{},{rate}", source.speed.get())
    };
    let rate = match crop {
        // In terms of the input's own dimensions, so the filter needs no
        // knowledge of how big the source is and stays right when the asset is
        // replaced by a bigger capture of the same thing.
        Some(crop) => format!(
            "{rate},crop=iw*{}:ih*{}:iw*{}:ih*{}",
            crop.width, crop.height, crop.x, crop.y
        ),
        None => rate,
    };
    let width = settings.resolution.width();
    let height = settings.resolution.height();
    let scale = |arguments: &str| resample(arguments, source.has_alpha);
    match fitting {
        // Explicit numbers rather than `force_original_aspect_ratio=decrease`,
        // because the buffer reading this pipe is sized from the same
        // rectangle: the two have to agree exactly, and the only way to be sure
        // they do is for one of them to have decided it.
        Fitting::Fit(fitted) => {
            let scale = scale(&format!("{}:{}", fitted.width(), fitted.height()));
            format!("{rate},{scale},format=rgba")
        }
        Fitting::FitPadded => {
            let scale = scale(&format!(
                "{width}:{height}:force_original_aspect_ratio=decrease"
            ));
            format!(
                "{rate},{scale},\
                 format=rgba,\
                 pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black@0.0"
            )
        }
        Fitting::Fill => {
            let scale = scale(&format!(
                "{width}:{height}:force_original_aspect_ratio=increase"
            ));
            format!("{rate},{scale},format=rgba,crop={width}:{height}")
        }
        // Nothing is resampled, so there is nothing alpha could be smeared by.
        Fitting::Native(_) => format!("{rate},format=rgba"),
    }
}

/// One `scale`, with the premultiply a transparent source needs around it.
///
/// swscale resamples each channel on its own, in **straight** alpha, where the
/// colour of a fully transparent pixel means nothing and is almost always
/// black. Average an opaque white pixel with one of those and the alpha comes
/// out right — half coverage — while the colour comes out grey, which on the
/// edge of a logo is a dark rim all the way round. Premultiplying first makes
/// the transparent pixel's colour weigh nothing, which is the whole of the fix.
/// The compositor already does this for the same reason before it hands a layer
/// to the rasteriser; this is the same care at the other end of the pipe.
///
/// **Only when the source really has alpha.** Wrapping an opaque source is not
/// a no-op: `premultiply` accepts only alpha-capable pixel formats, so a
/// `yuv420p` source gets a different conversion path into swscale and its
/// pixels move by a level or two. They are small differences, inside every
/// tolerance the golden gate uses — and they are differences to every frame of
/// every opaque source in the project, bought for a source that has no alpha to
/// protect.
fn resample(arguments: &str, has_alpha: bool) -> String {
    if has_alpha {
        format!("premultiply=inplace=1,scale={arguments},unpremultiply=inplace=1")
    } else {
        format!("scale={arguments}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::Fps;

    fn source(fitting: Fitting, has_alpha: bool) -> Source {
        Source {
            file: PathBuf::from("shot.mp4"),
            still: false,
            seek_seconds: 0.0,
            speed: Speed::NORMAL,
            frames: 30,
            fitting,
            has_alpha,
            crop: None,
        }
    }

    fn settings() -> RenderSettings {
        RenderSettings::new(
            Resolution::new(64, 64).expect("64x64 is a resolution"),
            Fps::THIRTY,
        )
    }

    fn filter(fitting: Fitting, has_alpha: bool) -> String {
        video_filter(&settings(), &source(fitting, has_alpha))
    }

    fn fitted() -> Fitting {
        Fitting::Fit(Resolution::new(64, 32).expect("64x32 is a resolution"))
    }

    /// The acceptance criterion of the change that added the premultiply: a
    /// source with no alpha — which is every opaque one, and every one nobody
    /// has probed — reaches ffmpeg with the chain it reached ffmpeg with
    /// before. Every golden reference in the repository was blessed against
    /// these exact strings.
    #[test]
    fn a_source_without_alpha_gets_the_chain_it_always_got() {
        assert_eq!(filter(fitted(), false), "fps=30/1,scale=64:32,format=rgba");
        assert_eq!(
            filter(Fitting::FitPadded, false),
            "fps=30/1,scale=64:64:force_original_aspect_ratio=decrease,format=rgba,\
             pad=64:64:(ow-iw)/2:(oh-ih)/2:color=black@0.0"
        );
        assert_eq!(
            filter(Fitting::Fill, false),
            "fps=30/1,scale=64:64:force_original_aspect_ratio=increase,format=rgba,crop=64:64"
        );
    }

    #[test]
    fn a_source_with_alpha_is_premultiplied_around_every_scale() {
        assert_eq!(
            filter(fitted(), true),
            "fps=30/1,premultiply=inplace=1,scale=64:32,unpremultiply=inplace=1,format=rgba"
        );
        assert!(filter(Fitting::FitPadded, true).contains(
            "premultiply=inplace=1,scale=64:64:force_original_aspect_ratio=decrease,\
             unpremultiply=inplace=1"
        ));
        assert!(filter(Fitting::Fill, true).contains(
            "premultiply=inplace=1,scale=64:64:force_original_aspect_ratio=increase,\
             unpremultiply=inplace=1"
        ));
    }

    fn holding(file: &str) -> Vec<String> {
        let mut command = Command::new("ffmpeg");
        hold(&mut command, Path::new(file), "30/1");
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    /// The still path every golden reference in the repository was blessed
    /// against. A png reaching ffmpeg any other way is a changed render.
    #[test]
    fn an_image2_still_is_held_by_the_demuxer_that_is_cheapest() {
        assert_eq!(holding("card.png"), ["-loop", "1", "-framerate", "30/1"]);
        assert_eq!(holding("CARD.JPEG"), ["-loop", "1", "-framerate", "30/1"]);
    }

    /// `-loop` is not an option these demuxers have, so asking for it is how a
    /// gif failed to render at all rather than how it was held.
    #[test]
    fn a_format_with_its_own_frames_is_held_by_looping_the_container() {
        assert_eq!(holding("reaction.gif"), ["-stream_loop", "-1"]);
        assert_eq!(holding("shot.avif"), ["-stream_loop", "-1"]);
    }

    /// Unrecognised takes the mechanism that works everywhere: slower is
    /// recoverable, and refusing to decode is not.
    #[test]
    fn a_still_nothing_recognises_is_held_the_general_way() {
        assert_eq!(holding("card"), ["-stream_loop", "-1"]);
        assert_eq!(holding("card.heic"), ["-stream_loop", "-1"]);
    }

    /// `native` resamples nothing, so there is nothing for alpha to be smeared
    /// by and no filter pair to pay for.
    #[test]
    fn a_native_source_is_untouched_either_way() {
        let native = Fitting::Native(Resolution::new(32, 32).expect("32x32 is a resolution"));
        assert_eq!(filter(native, true), "fps=30/1,format=rgba");
        assert_eq!(filter(native, false), filter(native, true));
    }
}
