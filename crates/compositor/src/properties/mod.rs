//! The properties this compositor animates, and where their names live.
//!
//! `scorsese-core` deliberately does not know that `opacity` means anything —
//! a property path there is an opaque string, and the keyframe evaluator works
//! on any numeric property. **This module is where those strings acquire
//! meaning**, which is what keeps the generality rule intact: adding an
//! animatable property is a change here, next to the code that implements it,
//! and never a change to the format or the model.
//!
//! A keyframe track naming something not listed here is **ignored**. That is a
//! deliberate non-failure: a project authored against a newer compositor must
//! still render on an older one, and an unknown property must never be able to
//! fail a render.
//!
//! It is also why a typo would otherwise do nothing quietly, so [`ANIMATED`]
//! publishes what these names are. It sits here, in the same file as the match
//! that resolves them, so the list cannot drift from the code — and in this
//! crate rather than in `scorsese-core`, because the moment core holds a list
//! of known properties, adding one becomes a core change and the generality
//! rule is gone.

use scorsese_core::{Blend, ChromaKey, Clip, Frames, Glow, Grade, KeyframeTrack, Shadow, Vhs};

use crate::registry::Property;
use crate::shape::Trace;
use crate::text::Sweep;

mod fades;
mod reading;
mod text;

pub use fades::{fade_in, fade_out};

/// The property paths this compositor resolves.
pub mod path;

/// What this compositor animates, and what animating it does.
///
/// The vocabulary itself, next to the [`Properties::at`] match that gives each
/// name meaning: a property added there without being added here is a property
/// nothing can tell you about, and one added here without being implemented
/// there is a promise nothing keeps. Adding both is one edit in one file.
pub const ANIMATED: &[Property] = &[
    Property {
        path: path::OPACITY,
        describes: "how solid the layer is",
    },
    Property {
        path: path::BLUR,
        describes: "how far the layer's own pixels are softened, as a fraction of its own height",
    },
    Property {
        path: path::ABERRATION,
        describes: "how far the layer's colour channels are pulled apart from its centre outward, \
                    as a fraction of its own height",
    },
    Property {
        path: path::KEY_TOLERANCE,
        describes: "how far a pixel's colour may sit from the keyed screen colour and still be \
                    keyed out",
    },
    Property {
        path: path::KEY_SOFTNESS,
        describes: "how wide the ramp from screen to subject is, outward from the tolerance",
    },
    Property {
        path: path::POSITION_X,
        describes: "how far right the layer is moved, as a fraction of the raster's width",
    },
    Property {
        path: path::POSITION_Y,
        describes: "how far down the layer is moved, as a fraction of the raster's height",
    },
    Property {
        path: path::SCALE_X,
        describes: "the layer's width, as a multiplier about its own origin",
    },
    Property {
        path: path::SCALE_Y,
        describes: "the layer's height, as a multiplier about its own origin",
    },
    Property {
        path: path::ROTATION,
        describes: "how far the layer is turned clockwise about its own origin, in degrees",
    },
    Property {
        path: path::FLIP_X,
        describes: "how far the layer is turned about its own horizontal axis, in degrees",
    },
    Property {
        path: path::FLIP_Y,
        describes: "how far the layer is turned about its own vertical axis, in degrees",
    },
    Property {
        path: path::SATURATION,
        describes: "how much colour the layer has, as a multiplier about each pixel's own grey",
    },
    Property {
        path: path::TEMPERATURE,
        describes: "which way the layer's whites lean: negative cooler, positive warmer",
    },
    Property {
        path: path::BRIGHTNESS,
        describes: "how much light is added to the layer, as an offset",
    },
    Property {
        path: path::CONTRAST,
        describes: "how steep the layer's range is about mid-grey",
    },
    Property {
        path: path::VIGNETTE,
        describes: "how much the layer's own corners are darkened",
    },
    Property {
        path: path::GRAIN,
        describes: "how much grain is laid over the layer, strongest through the midtones",
    },
    Property {
        path: path::CHROMA_BLEED,
        describes: "how far the tape smeared the layer's colour sideways, as a fraction of its \
                    own width",
    },
    Property {
        path: path::TAPE_NOISE,
        describes: "how much snow the tape laid over the layer, on its colour as well as its \
                    brightness",
    },
    Property {
        path: path::SCANLINES,
        describes: "how dark the tape's alternate lines are",
    },
    Property {
        path: path::JITTER,
        describes: "how far the tape's tracking wobbles the layer sideways, as a fraction of its \
                    own width",
    },
    Property {
        path: path::HEAD_SWITCH,
        describes: "how torn the band at the bottom of the layer is, where the tape's heads \
                    hand over",
    },
    Property {
        path: path::TRIM_START,
        describes: "where a shape's drawn line starts, as a fraction of its outline's length",
    },
    Property {
        path: path::TRIM_END,
        describes: "where a shape's drawn line ends, as a fraction of its outline's length",
    },
    Property {
        path: path::DASH_OFFSET,
        describes: "how far a dashed shape's pattern has moved along its line, as a fraction \
                    of the raster's height",
    },
    Property {
        path: path::REVEAL,
        describes: "how much of a text layer has arrived, piece by piece: 0 none, 1 all",
    },
    Property {
        path: path::NUMBER,
        describes: "the figure a text layer writes where its text says {n}",
    },
    Property {
        path: path::SHADOW_OPACITY,
        describes: "how dark the layer's drop shadow is, clamped to 0-1; nothing without a shadow",
    },
    Property {
        path: path::GLOW_RADIUS,
        describes: "how far the layer's glow reaches, as a fraction of its own height; nothing \
                    without a glow",
    },
    Property {
        path: path::GLOW_INTENSITY,
        describes: "how bright the layer's glow is, clamped to 0-4; nothing without a glow",
    },
];

/// What a layer looks like at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Properties {
    /// Offset from where the layer naturally sits, as a fraction of the canvas
    /// — x of its width, y of its height. Resolution is a render setting, so a
    /// layer nudged by a number of pixels would sit somewhere else the moment
    /// the same project was delivered at a different size.
    pub position: (f64, f64),
    /// Size multiplier about the layer's `origin`. `1.0` is natural size, so
    /// scaling a layer does not also move it.
    pub scale: (f64, f64),
    /// Turn about the layer's `origin`, in degrees, clockwise. The same pivot
    /// scale uses, because a card hinging on its left edge and a bar filling
    /// from one are the same request, and one point for both is what makes
    /// them so.
    pub rotation: f64,
    /// Turn about the layer's own axes, in degrees — `.0` about its
    /// **horizontal** axis and `.1` about its **vertical** one, which is the
    /// page-turn. `0.0` is face on, `180.0` is face on and mirrored, which is
    /// what the back of a card looks like.
    ///
    /// Flat, not perspective: the layer squashes along the axis it is turning
    /// about, and the near edge does not grow. A projective warp is not one
    /// affine matrix, and this reads correctly as a card turning without one.
    pub flip: (f64, f64),
    /// `0.0` invisible, `1.0` solid.
    pub opacity: f64,
    /// How far the layer's own pixels are softened before it is placed, as a
    /// fraction of the layer's own **height**. `0.0` leaves it alone.
    ///
    /// **Of the layer's height and not the canvas's**, which is what makes the
    /// number mean the same softness at 1080p and at 4K — and what makes
    /// `scale` multiply the apparent blur, since the softening happens on
    /// these pixels before the transform above places them.
    pub blur: f64,
    /// How far the layer's colour channels are pulled apart before it is
    /// placed, as a fraction of the layer's own **height** at its top and
    /// bottom edges. `0.0` leaves them exactly on top of one another.
    ///
    /// Radial from the layer's own centre, so it is zero in the middle and
    /// worst at the corners — which is what makes it read as a lens rather
    /// than as a misregistration. Of the layer's height and not the canvas's,
    /// like [`Properties::blur`], so the same number is the same fringing at
    /// 1080p and at 4K.
    pub aberration: f64,
    /// Which of the layer's pixels are not there, or `None` — which is almost
    /// every layer — for a picture whose only alpha is the alpha it arrived
    /// with.
    ///
    /// **The first thing that happens to the pixels**, before the grade below
    /// and so before everything: a grade shifts the screen's colour, and a key
    /// run afterwards would be aimed at a colour that is no longer there.
    pub chroma_key: Option<ChromaKey>,
    /// The colour treatment applied to the layer's own pixels, before any of
    /// the geometry above.
    ///
    /// **Before**, and that is the whole reason it lives on the layer rather
    /// than on the frame: a vignette is measured from the layer's own centre
    /// and a saturation applies to the layer's own pixels, so both have to
    /// happen while the layer is still a rectangle of its own rather than a
    /// contribution to somebody else's canvas.
    pub grade: Grade,
    /// The tape this layer was recorded onto, applied after everything above.
    ///
    /// **Last, because it is the recorder.** The grade is what the camera saw,
    /// the blur is its focus and the aberration is its glass; a tape is what
    /// held the result, so it goes over the finished picture rather than under
    /// it. It is also the only stage that displaces whole rows, which is the
    /// other reason it is where it is — see [`crate::CpuCompositor`]'s scratch.
    pub vhs: Vhs,
    /// Where this layer's tape noise, wobble and tear start, at this instant.
    ///
    /// **This is how the frame index reaches the tape**, and it is
    /// [`Properties::grain_seed`]'s argument a second time: a compositor draws
    /// one moment and animates nothing itself, so time reaches it only through
    /// this struct. The tape needs it more than the grade does — the wobble and
    /// the tear vary over time as well as down the picture, so both would be
    /// still pictures without it.
    ///
    /// **Zero unless there is a tape**, for the reason the grain's seed is zero
    /// unless there is grain: a seed nothing reads says nothing about the layer,
    /// and resolving one anyway would make two instants of an untaped clip
    /// compare unequal over a number neither of them uses.
    pub vhs_seed: u64,
    /// Where this layer's grain starts, at this instant — the noise field's
    /// seed, and nothing an author writes.
    ///
    /// **This is how the frame index reaches the grade.** A compositor draws
    /// one moment and animates nothing itself, so time reaches it only through
    /// this struct; the noise field's origin is part of what a layer looks like
    /// at an instant exactly as its opacity is. [`Properties::at`] folds the
    /// clip's id and its elapsed frame into it, which is the one place both are
    /// known: from the id, so two clips of the same footage never carry the
    /// same grain; from the frame, so the noise moves; and from nothing else,
    /// so a frame never depends on a frame drawn before it.
    ///
    /// **Zero unless [`Grade::grain`] is above zero**, and that is not a
    /// default so much as the honest value: a seed for a noise field nobody
    /// draws says nothing about the layer, and resolving one anyway would make
    /// two instants of an ungrained clip compare unequal over a number neither
    /// of them uses.
    pub grain_seed: u64,
    /// How much of a shape's line is drawn, and where its dashes are. Read
    /// only by a shape layer; the whole line on anything else.
    pub trace: Trace,
    /// Where a text layer's reveal is at this instant — [`Sweep::DONE`], all of
    /// it shown, unless a `reveal` track says otherwise. Read by whoever draws
    /// the layer's glyphs, and by nothing that composites it.
    pub sweep: Sweep,
    /// The figure a text layer's counter shows at this instant, or `None` for
    /// the value its own `number` block states. Read where the glyphs are
    /// drawn, like [`Properties::sweep`].
    pub number: Option<f64>,
    /// The drop shadow the layer casts, or `None`. Drawn from the layer's
    /// finished picture — after every stage above — and under it.
    pub shadow: Option<Shadow>,
    /// The halo of light the layer gives off, or `None`. Drawn from the same
    /// finished picture, between the shadow and the layer.
    pub glow: Option<Glow>,
    /// How the layer — its shadow and glow with it — lands on the canvas.
    pub blend: Blend,
}

impl Default for Properties {
    /// The layer exactly as it arrived: where it sits, its own size, facing
    /// front, solid.
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            scale: (1.0, 1.0),
            rotation: 0.0,
            flip: (0.0, 0.0),
            opacity: 1.0,
            blur: 0.0,
            aberration: 0.0,
            chroma_key: None,
            grade: Grade::NEUTRAL,
            vhs: Vhs::NONE,
            vhs_seed: 0,
            grain_seed: 0,
            trace: Trace::WHOLE,
            sweep: Sweep::DONE,
            number: None,
            shadow: None,
            glow: None,
            blend: Blend::Normal,
        }
    }
}

impl Properties {
    /// Resolves a clip's properties at `t`, in frames from the clip's own
    /// start.
    ///
    /// Anything the clip does not animate keeps its default, so a clip with no
    /// keyframes at all composites as a plain copy.
    ///
    /// The clip rather than its keyframes alone, because [`Grade`],
    /// [`Clip::blur`] and [`Clip::aberration`] are the properties that are
    /// *both* a field and animatable. The fields are what this starts from; a
    /// `grade.*`, `blur` or `aberration` track then takes that property over
    /// for the whole clip, the same way a track overrides the default for
    /// every other property here — which are animated or nothing.
    pub fn at(clip: &Clip, t: Frames) -> Self {
        Self::resolve(
            clip.id.as_str(),
            Self {
                grade: clip.grade,
                blur: clip.blur,
                aberration: clip.aberration,
                chroma_key: clip.chroma_key,
                vhs: clip.vhs,
                shadow: clip.shadow,
                glow: clip.glow,
                blend: clip.blend,
                ..Self::default()
            },
            &clip.keyframes,
            t,
        )
    }

    /// The same, from a baseline and a set of tracks — for callers that have
    /// tracks without a clip around them, which in practice means tests.
    ///
    /// The baseline is a whole [`Properties`] rather than the handful of fields
    /// a clip actually carries, and that is what stops this signature growing a
    /// positional `f64` every time a property becomes a field as well: a caller
    /// writes the one it means and takes [`Properties::default`] for the rest,
    /// which is the idiom every other caller of this type already uses.
    pub fn over(baseline: Self, tracks: &[KeyframeTrack], t: Frames) -> Self {
        // The empty id is the honest one for a caller with no clip: the grain
        // still animates, because `t` is here, and every such layer shares one
        // noise field, because there is nothing to tell them apart by.
        Self::resolve("", baseline, tracks, t)
    }

    /// Both of the above, which differ only in whether there is a clip to name.
    fn resolve(clip: &str, baseline: Self, tracks: &[KeyframeTrack], t: Frames) -> Self {
        let mut properties = baseline;
        for track in tracks {
            let Some(between) = track.between(t) else {
                continue;
            };
            let value = between.eased();
            match track.property.as_str() {
                path::REVEAL => properties.sweep = text::sweep(between),
                path::NUMBER => properties.number = Some(text::count(between)),
                path::OPACITY => properties.opacity = value,
                // Only when there is a key: a tolerance without a screen colour
                // is a number about nothing, and inventing a colour to hang it
                // on would key whatever that invention happened to be.
                path::KEY_TOLERANCE => {
                    if let Some(key) = properties.chroma_key.as_mut() {
                        key.tolerance = value;
                    }
                }
                path::KEY_SOFTNESS => {
                    if let Some(key) = properties.chroma_key.as_mut() {
                        key.softness = value;
                    }
                }
                path::BLUR => properties.blur = value,
                path::ABERRATION => properties.aberration = value,
                path::POSITION_X => properties.position.0 = value,
                path::POSITION_Y => properties.position.1 = value,
                path::SCALE_X => properties.scale.0 = value,
                path::SCALE_Y => properties.scale.1 = value,
                path::ROTATION => properties.rotation = value,
                path::FLIP_X => properties.flip.0 = value,
                path::FLIP_Y => properties.flip.1 = value,
                path::SATURATION => properties.grade.saturation = value,
                path::TEMPERATURE => properties.grade.temperature = value,
                path::BRIGHTNESS => properties.grade.brightness = value,
                path::CONTRAST => properties.grade.contrast = value,
                path::VIGNETTE => properties.grade.vignette = value,
                path::GRAIN => properties.grade.grain = value,
                path::CHROMA_BLEED => properties.vhs.chroma_bleed = value,
                path::TAPE_NOISE => properties.vhs.noise = value,
                path::SCANLINES => properties.vhs.scanlines = value,
                path::JITTER => properties.vhs.jitter = value,
                path::HEAD_SWITCH => properties.vhs.head_switch = value,
                path::TRIM_START => properties.trace.trim_start = value,
                path::TRIM_END => properties.trace.trim_end = value,
                path::DASH_OFFSET => properties.trace.dash_offset = value,
                // Only when there is one, for the key's reason above: a shadow
                // or a glow invented to hang a number on would be a colour and
                // an offset nobody chose.
                path::SHADOW_OPACITY => {
                    if let Some(shadow) = properties.shadow.as_mut() {
                        shadow.opacity = value;
                    }
                }
                path::GLOW_RADIUS => {
                    if let Some(glow) = properties.glow.as_mut() {
                        glow.radius = value;
                    }
                }
                path::GLOW_INTENSITY => {
                    if let Some(glow) = properties.glow.as_mut() {
                        glow.intensity = value;
                    }
                }
                _ => {}
            }
        }
        // After the tracks and not before them, because a `grade.grain` track
        // is one of the ways grain gets turned on — and only when there is
        // grain, so a layer without any stays exactly its own defaults.
        if properties.grade.grain > 0.0 {
            properties.grain_seed = crate::grain::seed(clip, t);
        }
        // And the same again for the tape. The same instant's seed, and a
        // separate field of this struct because either effect can be present
        // without the other — a tape reading the grain's seed would be a tape
        // that only wobbled on graded clips. What keeps a clip carrying both
        // from wearing one speckle twice is `grain::field`, which the tape
        // splits this into on the way in.
        if !properties.vhs.is_none() {
            properties.vhs_seed = crate::grain::seed(clip, t);
        }
        properties
    }
}
