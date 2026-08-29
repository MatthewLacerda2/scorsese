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

use scorsese_core::{Clip, Easing, Frames, Grade, Keyframe, KeyframeTrack, PropertyPath};

use crate::registry::Property;

/// The property paths this compositor resolves.
pub mod path {
    /// How solid the layer is: `0.0` invisible, `1.0` opaque.
    pub const OPACITY: &str = "opacity";
    /// How far the layer's own pixels are softened, as a fraction of the
    /// layer's own **height**. `0.0` untouched, higher is blurrier.
    ///
    /// Animated as `blur` and not `grade.blur`: a grade is the closed set of
    /// colour properties, each of which reads one pixel and writes one, and
    /// this one reads a neighbourhood.
    pub const BLUR: &str = "blur";
    /// How far the layer's colour channels are pulled apart by the lens, as a
    /// fraction of the layer's own **height** at its top and bottom edges.
    /// `0.0` is glass nobody ever looked through; higher fringes harder.
    ///
    /// Animated as `aberration` and not `grade.aberration`, for [`BLUR`]'s
    /// reason and more plainly than blur has it: this reads *three* source
    /// pixels to write one, where every field of a grade reads the one it is
    /// writing.
    pub const ABERRATION: &str = "aberration";
    /// Horizontal offset from where the layer naturally sits, as a fraction of
    /// the canvas **width**: `0.25` is a quarter of the way across it.
    pub const POSITION_X: &str = "transform.position.x";
    /// Vertical offset, as a fraction of the canvas **height**. Positive is
    /// down, as on the raster.
    pub const POSITION_Y: &str = "transform.position.y";
    /// Horizontal size multiplier about the layer's `origin`, which is its
    /// own centre unless the clip named another point.
    pub const SCALE_X: &str = "transform.scale.x";
    /// Vertical size multiplier about the layer's `origin`, which is its own
    /// centre unless the clip named another point.
    pub const SCALE_Y: &str = "transform.scale.y";
    /// Turn about the layer's `origin`, in degrees. **Positive is
    /// clockwise** — nobody should have to render a frame to find that out.
    pub const ROTATION: &str = "transform.rotation";
    /// Turn about the layer's own **horizontal** axis, in degrees: the top edge
    /// swinging toward you. `0` is face on, `90` edge on and so invisible,
    /// `180` face on again and mirrored top to bottom.
    ///
    /// The name is the axis turned **about**, not the direction the picture
    /// appears to move — the same convention [`ROTATION`] uses, and said
    /// outright here because nobody should have to render a frame to find out
    /// which way a flip goes.
    pub const FLIP_X: &str = "transform.flip.x";
    /// Turn about the layer's own **vertical** axis, in degrees: the page-turn,
    /// one side edge swinging toward you. `0` is face on, `90` edge on and so
    /// invisible, `180` face on again and mirrored left to right.
    ///
    /// Named for the axis turned **about**, so this is the one some people
    /// would call "flipping horizontally" — see [`FLIP_X`] for why the
    /// convention is worth stating rather than guessing at.
    pub const FLIP_Y: &str = "transform.flip.y";
    /// How much colour, about each pixel's own grey. `1.0` untouched, `0.0`
    /// fully grey, above `1.0` oversaturated.
    pub const SATURATION: &str = "grade.saturation";
    /// Which way the whites lean. `0.0` untouched, negative cooler, positive
    /// warmer.
    pub const TEMPERATURE: &str = "grade.temperature";
    /// Light added, as an offset. `0.0` untouched, negative darker, positive
    /// lighter.
    pub const BRIGHTNESS: &str = "grade.brightness";
    /// How steep the range is about mid-grey. `1.0` untouched, below flattens,
    /// above steepens.
    pub const CONTRAST: &str = "grade.contrast";
    /// How much the layer's own corners are darkened. `0.0` none, `1.0` takes
    /// them to black.
    pub const VIGNETTE: &str = "grade.vignette";
    /// How much grain is laid over the layer. `0.0` none, `1.0` heaviest.
    ///
    /// Animated as `grade.grain` and not as a property of its own, unlike
    /// [`BLUR`]: grain reads one pixel and writes one pixel, which is the test
    /// for being part of a grade. That it also consults the frame is what makes
    /// it move, not what makes it something else.
    pub const GRAIN: &str = "grade.grain";
}

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
    /// The colour treatment applied to the layer's own pixels, before any of
    /// the geometry above.
    ///
    /// **Before**, and that is the whole reason it lives on the layer rather
    /// than on the frame: a vignette is measured from the layer's own centre
    /// and a saturation applies to the layer's own pixels, so both have to
    /// happen while the layer is still a rectangle of its own rather than a
    /// contribution to somebody else's canvas.
    pub grade: Grade,
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
            grade: Grade::NEUTRAL,
            grain_seed: 0,
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
            let Some(value) = track.value_at(t) else {
                continue;
            };
            match track.property.as_str() {
                path::OPACITY => properties.opacity = value,
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
                _ => {}
            }
        }
        // After the tracks and not before them, because a `grade.grain` track
        // is one of the ways grain gets turned on — and only when there is
        // grain, so a layer without any stays exactly its own defaults.
        if properties.grade.grain > 0.0 {
            properties.grain_seed = crate::grain::seed(clip, t);
        }
        properties
    }

    /// The multiplier each axis is actually drawn at, with the flips folded in.
    ///
    /// A flip is not a transform of its own. Turning a card about one of its
    /// own axes narrows what you see of it by `cos θ` along the *other* one and
    /// does nothing else — so the whole feature is a factor on the scale that
    /// was already being applied about the layer's own centre. That is also why
    /// there is no separate backface case anywhere: `cos 180°` is `−1`, a
    /// negative scale is a mirror, and a mirror is what the back of a card
    /// looks like.
    ///
    /// **The axes cross over, and they cross over here, once.** A turn about
    /// the vertical axis is what changes the horizontal extent. Getting that
    /// backwards is the obvious bug in this feature, and it is far cheaper to
    /// check in one function than at every place a scale is read.
    pub fn effective_scale(&self) -> (f64, f64) {
        (
            self.scale.0 * self.flip.1.to_radians().cos(),
            self.scale.1 * self.flip.0.to_radians().cos(),
        )
    }

    /// True when this layer would draw exactly its own pixels, unmoved and
    /// unblended — which lets a compositor copy rather than rasterise.
    pub fn is_identity(&self) -> bool {
        const EPSILON: f64 = 1e-9;
        // The effective scale rather than the authored one, so a layer flipped
        // to its back — scale `−1`, a mirror — is never mistaken for a copy,
        // while one turned the whole way round to `360°` correctly is one.
        let (scale_x, scale_y) = self.effective_scale();
        self.position.0.abs() < EPSILON
            && self.position.1.abs() < EPSILON
            && (scale_x - 1.0).abs() < EPSILON
            && (scale_y - 1.0).abs() < EPSILON
            // A turned layer is never a plain copy, however slight the turn.
            && self.rotation.abs() < EPSILON
            && (self.opacity - 1.0).abs() < EPSILON
            // A softened layer is not its own pixels either, and this is the
            // easiest of these to forget: a blurred clip that is otherwise
            // untouched satisfies every other line here, so leaving it out
            // would send exactly the commonest blur — one on a full-frame plate
            // with no transform on it — down the copy path and render it sharp.
            // A negative number is not a blur and softens nothing, so it copies
            // like zero does.
            && self.blur <= EPSILON
            // And a layer whose channels have been pulled apart is not its own
            // pixels either, for exactly the reason above: a plate with nothing
            // on it but an aberration satisfies every other line here, so
            // leaving it out would copy the source through and render the one
            // case this costs least to apply to with no fringing at all.
            && self.aberration <= EPSILON
            // A graded layer is not its own pixels, which is the whole point of
            // grading it. Left out, the copy path below would hand the ungraded
            // source straight to the canvas and the grade would silently do
            // nothing on exactly the clips it costs least to apply to.
            && self.grade.is_neutral()
    }

    /// True when the layer would contribute nothing, so it can be skipped
    /// rather than rasterised into oblivion.
    ///
    /// This is what makes an edge-on layer *genuinely absent*. At exactly `90°`
    /// the effective scale is `cos 90°`, which is zero to within a rounding
    /// error, and a rasteriser handed that would smear a line of colour down
    /// the middle of the frame instead of drawing nothing at all.
    pub fn is_invisible(&self) -> bool {
        const EPSILON: f64 = 1e-9;
        let (scale_x, scale_y) = self.effective_scale();
        self.opacity <= EPSILON || scale_x.abs() <= EPSILON || scale_y.abs() <= EPSILON
    }
}

/// Ramps a clip up from nothing over its first `duration` frames.
///
/// Sugar, and nothing but sugar: it writes ordinary opacity keyframes, which
/// stay visible, editable, and deletable like any others. There is no fade
/// machinery for a renderer to know about, which is why a fade composes with a
/// move or a zoom for free.
///
/// Linear, because a fade is the neutral case and a curve is the author's
/// choice — edit the `easing` on the keyframe it writes.
pub fn fade_in(clip: &mut Clip, duration: Frames) {
    let duration = duration.get().min(clip.duration.get());
    if duration == 0 {
        return;
    }
    set_opacity(clip, Frames::ZERO, 0.0);
    set_opacity(clip, Frames(duration), 1.0);
}

/// Ramps a clip down to nothing over its last `duration` frames.
///
/// The ramp reaches zero at the clip's end — the frame after its last — so the
/// picture is still just barely there on the final frame and goes out exactly
/// on the cut.
pub fn fade_out(clip: &mut Clip, duration: Frames) {
    let total = clip.duration.get();
    let duration = duration.get().min(total);
    if duration == 0 {
        return;
    }
    set_opacity(clip, Frames(total - duration), 1.0);
    set_opacity(clip, Frames(total), 0.0);
}

/// Writes one opacity keyframe, replacing any already at that time and keeping
/// the track sorted — which validation requires and the evaluator assumes.
fn set_opacity(clip: &mut Clip, t: Frames, value: f64) {
    let track = match clip
        .keyframes
        .iter_mut()
        .find(|track| track.property.as_str() == path::OPACITY)
    {
        Some(track) => track,
        None => {
            clip.keyframes.push(KeyframeTrack::new(
                PropertyPath::new(path::OPACITY),
                Vec::new(),
            ));
            clip.keyframes
                .last_mut()
                .expect("the track just pushed is there")
        }
    };
    let keyframe = Keyframe {
        t,
        value,
        easing: Easing::Linear,
    };
    match track.keyframes.binary_search_by_key(&t, |frame| frame.t) {
        Ok(at) => track.keyframes[at] = keyframe,
        Err(at) => track.keyframes.insert(at, keyframe),
    }
}
