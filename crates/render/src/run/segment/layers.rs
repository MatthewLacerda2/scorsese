//! Getting one layer of a segment ready: a pipe to pull frames from, or pixels
//! drawn once and then left alone.
//!
//! **Not every layer has a source.** A text asset carries its content in the
//! document, and a prompt with nothing generated has a slug card; both are
//! drawn once, here, because their pixels do not change over a clip and
//! re-drawing every glyph thirty times a second to get the same answer would
//! be waste. What is left — the layers that do have a source — is exactly the
//! set the producer reads in lockstep, one frame from each per output frame.

use scorsese_compositor::{Area, Frame};
use scorsese_core::{Anchor, AssetKind, Clip, Fps, Frames};

use crate::error::RenderError;
use crate::pipe::{Decoder, Fitting, Source};
use crate::plan::Shot;
use crate::report::{Note, StandIn};
use crate::shape;
use crate::slug::{self, Standing};
use crate::text::Painter;

use super::Pass;
use super::attach::{self, Following, Rect};

/// What one layer contributes to every frame of a segment.
pub(super) struct Slot {
    /// Where its pixels come from.
    pub(super) pixels: Pixels,
    /// Which edges of the frame its position is measured from. Fixed for the
    /// clip — an anchor is not animated — so it is read once here rather than
    /// worked out again for every frame.
    pub(super) anchor: Anchor,
    /// Where this layer's picture sits within its own raster, which is what an
    /// arrow attaches *to*. Worked out once: animation moves the layer, never
    /// the rectangle inside it.
    pub(super) rect: Rect,
}

/// The three ways a layer has pixels, and the difference is what a worker is
/// allowed to share.
pub(super) enum Pixels {
    /// Drawn once and held for the whole segment: a title, a colour, a slug
    /// card. Every frame of the segment reads the same buffer, so workers
    /// share it rather than each being handed a copy.
    Held(Frame),
    /// Read from a decoder into the frame job's own buffer at this index.
    /// Those pixels belong to one output frame, where a held layer's belong to
    /// the whole segment — which is why one is owned by the job and the other
    /// is not.
    Live(usize),
    /// Drawn afresh into the job's own buffer for every frame, because what it
    /// looks like depends on where *another* layer is at that instant.
    ///
    /// Only an attached arrow is ever this. Everything else that is drawn — a
    /// title, a colour, a box, an arrow between two fixed points — is the same
    /// pixels for the whole segment and is [`Pixels::Held`], which is what
    /// keeps the common case free of per-frame work.
    Drawn {
        /// Which of the job's buffers it is drawn into, counted after the
        /// decoded ones.
        at: usize,
        /// The arrow, and where each of its ends comes from.
        following: Box<Following>,
    },
}

impl Pass<'_> {
    /// Gets one layer ready. `live` is the index its buffer will take among the
    /// layers that are read, which is the same index its decoder takes — so
    /// the caller pushes both or neither.
    pub(super) fn begin(
        &self,
        shot: &Shot<'_>,
        painter: &mut Painter,
        frames: u64,
        live: usize,
        drawn: usize,
        segment: &[Shot<'_>],
        notes: &mut Vec<Note>,
    ) -> Result<(Slot, Option<Decoder>), RenderError> {
        let raster = self.settings.resolution;
        // Both of the drawn kinds are the size of the raster: a title is set at
        // whatever the render is, and a card is a panel of it.
        let held = |pixels: Frame, area: Area| {
            Ok((
                Slot {
                    pixels: Pixels::Held(pixels),
                    anchor: shot.clip.anchor,
                    rect: Rect {
                        source: raster,
                        area,
                        anchor: shot.clip.anchor,
                    },
                },
                None,
            ))
        };
        let blank = || Frame::black(raster);

        if shot.asset.kind == AssetKind::Text {
            let mut pixels = blank();
            painter.paint(&mut pixels, shot.asset, shot.clip.anchor, self.project_root)?;
            // The wrapped block, not the raster it is set on: an arrow attached
            // to a title has to meet the words, and the layer's own edges are
            // the frame's.
            let block = painter.block(shot.asset, shot.clip.anchor, self.project_root, raster)?;
            return held(pixels, block);
        }

        if shot.asset.kind == AssetKind::Color {
            let mut pixels = blank();
            // Validation requires the colour, so the default is unreachable
            // through a loaded project. White rather than transparent if a
            // caller ever builds one in memory: a layer that silently vanished
            // would be harder to notice than one that is plainly the wrong
            // colour.
            pixels.fill(shot.asset.color.unwrap_or_default());
            // A colour *is* the whole raster, so its rectangle is too.
            return held(pixels, Area::whole(raster));
        }

        if let Some(shape) = &shot.asset.shape {
            // An arrow with an end that follows a clip cannot be drawn once for
            // the segment: where it runs depends on where that clip is at each
            // instant. Everything else here is the same pixels throughout.
            if attach::is_attached(shape) {
                let following = attach::following(shape, segment);
                if following.is_none() {
                    notes.push(Note::ArrowUnattached {
                        clip: shot.clip.id.to_string(),
                    });
                }
                return Ok((
                    Slot {
                        pixels: match following {
                            Some(following) => Pixels::Drawn {
                                at: drawn,
                                following: Box::new(following),
                            },
                            // Its target is not on screen here, so there is
                            // nothing to point at and nothing to draw.
                            None => Pixels::Held(transparent(raster)),
                        },
                        anchor: shot.clip.anchor,
                        rect: Rect {
                            source: raster,
                            area: Area::whole(raster),
                            anchor: shot.clip.anchor,
                        },
                    },
                    None,
                ));
            }
            // The anchor reaches the drawing rather than the compositing. A
            // shape layer is the size of the raster, and a raster-sized layer
            // rests at the origin whatever its anchor — so where the box sits
            // *inside* it has to be decided while it is drawn, exactly as a
            // block of text's is.
            let mut pixels = blank();
            shape::paint(&mut pixels, shape, shot.clip.anchor);
            // A box's own box, not the raster it is drawn into — what a reader
            // sees, and the only rectangle an arrow could sensibly meet.
            let area =
                shape::area(shape, raster, shot.clip.anchor).unwrap_or_else(|| Area::whole(raster));
            return held(pixels, area);
        }

        let file = match slug::standing(shot, self.project_root)? {
            Standing::Media(file) => file,
            Standing::Card(absent) => {
                if absent.is_a_problem() {
                    notes.push(Note::GeneratedMissing {
                        clip: shot.clip.id.to_string(),
                        asset: shot.asset.id.to_string(),
                        stood_in: StandIn::SlugCard,
                    });
                }
                let mut pixels = blank();
                slug::paint(&mut pixels, shot.asset, absent);
                return held(pixels, Area::whole(raster));
            }
        };

        let decoder = Decoder::start(
            self.tools,
            &source_for(
                shot,
                file,
                self.plan.timeline_fps(),
                frames,
                self.sizes.fitting(shot, self.settings.resolution),
            ),
            &self.settings,
        )?;
        // A decoded picture fills the buffer it is read into, so its rectangle
        // is that buffer — which for a letterboxed `fit` is the picture and not
        // the raster, since the decode stage hands over the fitted rectangle
        // rather than padding it out first.
        let source = decoder.raster();
        Ok((
            Slot {
                pixels: Pixels::Live(live),
                anchor: shot.clip.anchor,
                rect: Rect {
                    source,
                    area: Area::whole(source),
                    anchor: shot.clip.anchor,
                },
            },
            Some(decoder),
        ))
    }
}

/// A raster with nothing on it — what an arrow whose target is absent
/// contributes.
fn transparent(resolution: scorsese_compositor::Resolution) -> Frame {
    let mut frame = Frame::black(resolution);
    frame.fill_transparent();
    frame
}

/// How far into its own clip a timeline frame is.
pub(super) fn elapsed(at: Frames, clip: &Clip) -> Frames {
    Frames(at.get().saturating_sub(clip.start.get()))
}

/// How to read a shot's media, given where it is.
fn source_for(
    shot: &Shot<'_>,
    file: std::path::PathBuf,
    timeline_fps: Fps,
    frames: u64,
    fitting: Fitting,
) -> Source {
    Source {
        file,
        // A still has no timeline of its own: it is held for the clip's
        // length rather than played, so there is nothing to seek into.
        still: shot.asset.kind == AssetKind::Image,
        seek_seconds: timeline_fps.seconds_at(shot.source_in),
        speed: shot.clip.speed,
        frames,
        fitting,
        // Read off the document, which is where a probe writes it. An asset
        // nobody probed reads as opaque — the same answer, and the same filter
        // chain, as before there was a field to read.
        has_alpha: shot
            .asset
            .media
            .and_then(|media| media.has_alpha)
            .unwrap_or(false),
        crop: shot.clip.crop,
    }
}
