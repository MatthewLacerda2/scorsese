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
/// How far a pixel's colour may sit from the keyed screen colour and still
/// be screen. `0.0` keys only an exact match; higher takes more with it.
///
/// **Does nothing on a clip with no `chroma_key`**, and that is not the
/// ignore-an-unknown-property rule: the path is known and resolved, there
/// is simply no key for it to be a tolerance *of*. A key needs a colour,
/// and a colour is not a number a track can carry.
pub const KEY_TOLERANCE: &str = "chroma_key.tolerance";
/// How wide the ramp from screen to subject is, measured outward from
/// [`KEY_TOLERANCE`]. `0.0` is a hard cutout. Does nothing without a key,
/// for the reason above.
pub const KEY_SOFTNESS: &str = "chroma_key.softness";
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
/// How far the tape smeared colour sideways, as a fraction of the layer's
/// own **width**. `0.0` none, `1.0` heaviest. Nothing at all in `mono`,
/// where there is no chroma path to smear.
pub const CHROMA_BLEED: &str = "vhs.chroma_bleed";
/// How much snow the tape laid over the layer. `0.0` none, `1.0` heaviest.
///
/// The tape's noise rather than the emulsion's, and a clip may carry both:
/// this one speckles the colour differences as well as the luma, which is
/// what makes tape noise coloured where [`GRAIN`] is not.
pub const TAPE_NOISE: &str = "vhs.noise";
/// How dark the tape's alternate lines are. `0.0` none, `1.0` darkest.
pub const SCANLINES: &str = "vhs.scanlines";
/// How far the tracking wobbles, as a fraction of the layer's own
/// **width** — the one measurement here that is, because a row is
/// displaced along itself. `0.0` holds still.
pub const JITTER: &str = "vhs.jitter";
/// How torn the band at the bottom of the picture is, where the tape's
/// heads hand over. `0.0` leaves the bottom of frame alone.
pub const HEAD_SWITCH: &str = "vhs.head_switch";
/// Where a shape's drawn line starts, as a fraction of the outline's
/// length. `0.0` its start. Clamped to `0.0..=1.0` when drawn.
///
/// Under `shape.` because it belongs to what a shape asset draws rather
/// than to the layer: on anything else nothing reads it.
pub const TRIM_START: &str = "shape.trim_start";
/// Where a shape's drawn line ends, as a fraction of the outline's length.
/// `1.0` all the way; `0.0 → 1.0` is a line drawing itself on.
pub const TRIM_END: &str = "shape.trim_end";
/// How far a dashed shape's pattern has moved along its line, toward the
/// end, as a fraction of the raster's **height**. Increasing it makes the
/// dashes flow — "marching ants".
pub const DASH_OFFSET: &str = "shape.dash_offset";
/// How much of a text layer has arrived, `0.0` none and `1.0` all, piece
/// by piece as its `reveal` block cuts it. Its keyframe easing shapes each
/// piece's entrance rather than the sweep across them. Nothing on a layer
/// that is not text.
pub const REVEAL: &str = "reveal";
/// The figure a text layer writes where its text says `{n}`, never
/// travelling past the keyframes either side of it. Nothing on a text with
/// no `number` block, or on a layer that is not text.
pub const NUMBER: &str = "number";
/// How dark the clip's drop shadow is: `0.0` none, `1.0` the full colour
/// wherever the layer is solid. **Clamped to that range where it is
/// drawn**, so an overshooting easing darkens to full and no further.
///
/// Does nothing on a clip with no `shadow`, for [`KEY_TOLERANCE`]'s
/// reason: a shadow needs a colour and an offset, and neither is a number
/// a track can carry.
pub const SHADOW_OPACITY: &str = "shadow.opacity";
/// How far the clip's glow reaches, as a fraction of the layer's own
/// **height** — measured exactly as [`BLUR`] is. Below zero is no reach at
/// all, and it never reaches further than the layer is tall. Does nothing
/// on a clip with no `glow`.
pub const GLOW_RADIUS: &str = "glow.radius";
/// How bright the clip's glow is: `0.0` none, `1.0` the layer's own light
/// spread out, more is more. **Clamped to `0.0` –
/// [`crate::MAX_GLOW_INTENSITY`] where it is drawn**, so a spring easing
/// on a pulse overshoots into a brighter flash rather than a negative one.
/// Does nothing on a clip with no `glow`.
pub const GLOW_INTENSITY: &str = "glow.intensity";
/// How far along the arrow it follows a clip is, as a fraction of the
/// arrow's **length**: `0.0` its tail, `1.0` its head. Clamped to
/// `0.0..=1.0` when placed, so an overshooting easing rests at an end.
///
/// Under `follow.` because it belongs to the clip's `follow` block: on a
/// clip without one nothing reads it.
pub const FOLLOW_PROGRESS: &str = "follow.progress";
