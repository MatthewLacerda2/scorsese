//! Getting one layer of a segment ready: a pipe to pull frames from, or pixels
//! drawn once and then left alone.
//!
//! **Not every layer has a source.** A text asset carries its content in the
//! document, and a prompt with nothing generated has a slug card; both are
//! drawn once, here, because their pixels do not change over a clip and
//! re-drawing every glyph thirty times a second to get the same answer would
//! be waste. What is left — the layers that do have a source — is exactly the
//! set the producer reads in lockstep, one frame from each per output frame.

use scorsese_compositor::shape::Trace;
use scorsese_compositor::text::Sweep;
use scorsese_compositor::{Area, Frame};
use scorsese_core::{Anchor, AssetKind, Fps, Origin, Rgba};

use crate::content;
use crate::error::RenderError;
use crate::pipe::{Decoder, Fitting, Source};
use crate::plan::Shot;
use crate::report::{Note, StandIn};
use crate::shape;
use crate::slug::{self, Standing};
use crate::symbol;
use crate::text::{Painter, Typing};

use super::Pass;
use super::attach::{self, Rect};
use super::redraw::Redraw;

/// What one layer contributes to every frame of a segment.
pub(super) struct Slot {
    /// Where its pixels come from.
    pub(super) pixels: Pixels,
    /// Which edges of the frame its position is measured from. Fixed for the
    /// clip — an anchor is not animated — so it is read once here rather than
    /// worked out again for every frame.
    pub(super) anchor: Anchor,
    /// Which point of its own raster its transform turns about. Fixed for the
    /// clip, for the same reason the anchor is.
    pub(super) origin: Origin,
    /// Where this layer's picture sits within its own raster, which is what an
    /// arrow attaches *to*. Worked out once: animation moves the layer, never
    /// the rectangle inside it.
    pub(super) rect: Rect,
    /// The group this layer is drawn into, by its index among the segment's
    /// slots — `None` for a layer drawn straight onto the frame.
    pub(super) within: Option<usize>,
}

/// One shot as a segment draws it: the groups opened, and each shot knowing
/// which group it is drawn into.
///
/// The plan hands over a tree — a group clip with its members beneath it — and
/// a renderer wants a list, since a frame's buffers, decoders and properties
/// are all lists. So the tree is walked once per segment, **each group before
/// its members**, and the nesting survives as an index. That order is what the
/// drawing relies on: a group's members, and its nested groups, always come
/// after it, so drawing the groups from the last to the first finishes every
/// inner one before the one it is part of.
pub(super) struct Entry<'s, 'a> {
    /// The shot.
    pub(super) shot: &'s Shot<'a>,
    /// The group it is drawn into, by index in the same list.
    pub(super) within: Option<usize>,
}

/// A segment's layers with every group opened, in drawing order.
pub(super) fn open<'s, 'a>(layers: &'s [Shot<'a>]) -> Vec<Entry<'s, 'a>> {
    fn walk<'s, 'a>(shots: &'s [Shot<'a>], within: Option<usize>, out: &mut Vec<Entry<'s, 'a>>) {
        for shot in shots {
            let at = out.len();
            out.push(Entry { shot, within });
            walk(&shot.members, Some(at), out);
        }
    }
    let mut out = Vec::new();
    walk(layers, None, &mut out);
    out
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
    /// looks like changes from one instant to the next: an attached arrow,
    /// which depends on where *another* layer is, and a shape whose line is
    /// keyframed — see [`Redraw`].
    ///
    /// Only those. Everything else that is drawn — a title, a colour, a box,
    /// an arrow between two fixed points, a dashed border standing still — is
    /// the same pixels for the whole segment and is [`Pixels::Held`], which is
    /// what keeps the common case free of per-frame work. Text that reveals or
    /// counts is [`Pixels::Typed`].
    Drawn {
        /// Which of the job's buffers it is drawn into, counted after the
        /// decoded ones.
        at: usize,
        /// What it is drawn from, every frame.
        redraw: Box<Redraw>,
    },
    /// Text set afresh for every frame, because it is revealing or counting —
    /// which glyphs there are, and where, is what changes.
    ///
    /// Set by the worker compositing the frame rather than by the producer the
    /// way an arrow is: it depends on nothing but this layer's own properties,
    /// and setting type is the most expensive drawing there is, so it goes
    /// where the parallelism is.
    Typed {
        /// Which of the job's buffers it is drawn into, counted with
        /// [`Pixels::Drawn`]'s after the decoded ones.
        at: usize,
        /// The face, the style and the content, kept for the segment.
        typing: Box<Typing>,
    },
    /// A group: its members composited, every frame, into a transparent
    /// raster of the job's own, which is then this layer's picture.
    ///
    /// Per frame and never held, because a member may be footage or may move,
    /// and nothing is cached yet: a group costs one offscreen composite per
    /// frame, on top of what its members cost anyway.
    Composed {
        /// Which of the job's group canvases it is drawn into.
        at: usize,
    },
}

/// Where a layer sits among the ones being got ready, and what else is on
/// screen beside it.
///
/// One value rather than three arguments, because the three are only ever
/// passed together and two of them are indices into different lists — which is
/// exactly the pair a reader would otherwise have to keep straight at every
/// call.
pub(super) struct Among<'a> {
    /// How many layers before this one are read from a decoder, which is the
    /// buffer index this one takes if it is read too.
    pub(super) live: usize,
    /// How many before it are redrawn every frame, which is the buffer index it
    /// takes if it is one of those.
    pub(super) drawn: usize,
    /// How many before it are groups, which is the canvas index it takes if
    /// it is one.
    pub(super) composed: usize,
    /// The group it is drawn into, if any.
    pub(super) within: Option<usize>,
    /// Everything on screen in this stretch, groups opened — what an
    /// attachment is resolved against.
    pub(super) layers: &'a [Entry<'a, 'a>],
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
        among: Among<'_>,
        notes: &mut Vec<Note>,
    ) -> Result<(Slot, Option<Decoder>), RenderError> {
        let Among {
            live,
            drawn,
            composed,
            within,
            layers: segment,
        } = among;
        let raster = self.settings.resolution;
        // What this layer *shows* within its own raster, decided in one place
        // for the render and for anything asking where a clip landed — see
        // [`crate::content`]. Every drawn kind is the size of the raster: a
        // title is set at whatever the render is, and a card is a panel of it.
        let area = content::within(shot, painter, raster, self.project_root)?.or_whole(raster);
        // Every layer drawn at the raster's own size rests on it the same way;
        // only where its pixels come from differs.
        let at_raster = |pixels: Pixels| Slot {
            pixels,
            anchor: shot.clip.anchor,
            origin: shot.clip.origin,
            rect: Rect {
                source: raster,
                area,
                anchor: shot.clip.anchor,
                origin: shot.clip.origin,
            },
            within,
        };
        let held = |pixels: Frame| Ok((at_raster(Pixels::Held(pixels)), None));
        let blank = || Frame::black(raster);

        if shot.asset.kind == AssetKind::Group {
            // Nothing to draw once: the members are drawn into it every frame,
            // from their own slots, by the worker compositing the frame.
            return Ok((at_raster(Pixels::Composed { at: composed }), None));
        }

        if shot.asset.kind == AssetKind::Text {
            let typing = painter.typing(shot.asset, shot.clip, self.project_root, raster)?;
            // Drawn once as it stands whatever happens next: that is the
            // picture a still clip holds, and it is where a colour glyph drawn
            // short is noticed — once for the clip, not once for every frame
            // of a reveal.
            let mut pixels = blank();
            let unpaintable = typing.draw(&mut pixels, Sweep::DONE, None);
            notes.extend(unpaintable.into_iter().map(|glyph| Note::UnpaintableGlyph {
                clip: shot.clip.id.to_string(),
                asset: shot.asset.id.to_string(),
                wanted: glyph.wanted,
            }));
            if !typing.animates() {
                return held(pixels);
            }
            return Ok((
                at_raster(Pixels::Typed {
                    at: drawn,
                    typing: Box::new(typing),
                }),
                None,
            ));
        }

        if shot.asset.kind == AssetKind::Color {
            let mut pixels = blank();
            // Validation requires the colour, so the default is unreachable
            // through a loaded project. White rather than transparent if a
            // caller ever builds one in memory: a layer that silently vanished
            // would be harder to notice than one that is plainly the wrong
            // colour. A gradient is resolved against the whole raster, once
            // per segment like a flat colour — it does not move.
            match &shot.asset.color {
                Some(fill) => scorsese_compositor::gradient::paint(&mut pixels, fill),
                None => pixels.fill(Rgba::default()),
            }
            return held(pixels);
        }

        if let Some(icon) = &shot.asset.icon {
            // The anchor reaches the drawing rather than the compositing, for
            // the reason a shape's does: an icon layer is the size of the
            // raster, and a raster-sized layer rests at the origin whatever its
            // anchor says.
            let mut pixels = blank();
            symbol::paint(&mut pixels, icon, shot.clip.anchor);
            if symbol::is_unknown(icon) {
                notes.push(Note::UnknownIcon {
                    clip: shot.clip.id.to_string(),
                    asset: shot.asset.id.to_string(),
                    named: icon.name.clone(),
                });
            }
            // The rectangle is [`crate::content`]'s to decide and not this call
            // site's — the symbol's own square rather than the raster it is
            // drawn into, worked out once for the render and for anything asking
            // where the clip landed.
            return held(pixels);
        }

        if let Some(shape) = &shot.asset.shape {
            // An arrow with an end that follows a clip cannot be drawn once for
            // the segment: where it runs depends on where that clip is at each
            // instant. Everything else here is the same pixels throughout.
            if attach::is_attached(shape) {
                let following = attach::following(shape, segment, within);
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
                                redraw: Box::new(Redraw::Following(following)),
                            },
                            // Its target is not on screen here, so there is
                            // nothing to point at and nothing to draw.
                            None => Pixels::Held(transparent(raster)),
                        },
                        anchor: shot.clip.anchor,
                        origin: shot.clip.origin,
                        rect: Rect {
                            source: raster,
                            area: Area::whole(raster),
                            anchor: shot.clip.anchor,
                            origin: shot.clip.origin,
                        },
                        within,
                    },
                    None,
                ));
            }
            // A line whose trim or dash offset is keyframed is a different
            // picture at every instant, so it is drawn with the frame rather
            // than once for the segment. Its box is still its box: an arrow
            // attached to a border drawing itself on meets the whole box.
            if Trace::is_animated_by(&shot.clip.keyframes) {
                let redraw = Box::new(Redraw::Traced(shape.clone()));
                return Ok((at_raster(Pixels::Drawn { at: drawn, redraw }), None));
            }
            // The anchor reaches the drawing rather than the compositing. A
            // shape layer is the size of the raster, and a raster-sized layer
            // rests at the origin whatever its anchor — so where the box sits
            // *inside* it has to be decided while it is drawn, exactly as a
            // block of text's is.
            let mut pixels = blank();
            shape::paint(&mut pixels, shape, shot.clip.anchor, Trace::WHOLE);
            return held(pixels);
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
                return held(pixels);
            }
        };

        let (file, fitting) = self.previewed(shot, file);
        let decoder = Decoder::start(
            self.tools,
            &source_for(shot, file, self.plan.timeline_fps(), frames, fitting),
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
                origin: shot.clip.origin,
                rect: Rect {
                    source,
                    area: Area::whole(source),
                    anchor: shot.clip.anchor,
                    origin: shot.clip.origin,
                },
                within,
            },
            Some(decoder),
        ))
    }
}

impl Pass<'_> {
    /// Which file a shot is decoded from and how it meets the raster, once a
    /// preview has had its say ([`crate::preview`]).
    ///
    /// A delivery is untouched: its own file, its own fitting. A preview at a
    /// reduced quality reads the shot's proxy when there is one, and brings a
    /// `native` source down with the raster — asked for as an exact size,
    /// which is what [`Fitting::Fit`] already is, so the decoder scales a
    /// smaller proxy or a full-size original to the same rectangle.
    fn previewed(
        &self,
        shot: &Shot<'_>,
        file: std::path::PathBuf,
    ) -> (std::path::PathBuf, Fitting) {
        let fitting = self.sizes.fitting(shot, self.settings.resolution);
        let Some(preview) = self.preview else {
            return (file, fitting);
        };
        let quality = preview.quality();
        let fitting = match fitting {
            Fitting::Native(size) if quality.divisor() > 1 => Fitting::Fit(quality.shrink(size)),
            other => other,
        };
        let file = preview
            .proxy_for(shot.asset)
            .map_or(file, std::path::Path::to_path_buf);
        (file, fitting)
    }
}

/// A raster with nothing on it — what an arrow whose target is absent
/// contributes.
fn transparent(resolution: scorsese_compositor::Resolution) -> Frame {
    let mut frame = Frame::black(resolution);
    frame.fill_transparent();
    frame
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
