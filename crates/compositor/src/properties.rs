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

use scorsese_core::{Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

use crate::registry::Property;

/// The property paths this compositor resolves.
pub mod path {
    /// How solid the layer is: `0.0` invisible, `1.0` opaque.
    pub const OPACITY: &str = "opacity";
    /// Horizontal offset from where the layer naturally sits, in canvas pixels.
    pub const POSITION_X: &str = "transform.position.x";
    /// Vertical offset, in canvas pixels. Positive is down, as on the raster.
    pub const POSITION_Y: &str = "transform.position.y";
    /// Horizontal size multiplier about the layer's own centre.
    pub const SCALE_X: &str = "transform.scale.x";
    /// Vertical size multiplier about the layer's own centre.
    pub const SCALE_Y: &str = "transform.scale.y";
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
        path: path::POSITION_X,
        describes: "how far right the layer is moved, in output pixels",
    },
    Property {
        path: path::POSITION_Y,
        describes: "how far down the layer is moved, in output pixels",
    },
    Property {
        path: path::SCALE_X,
        describes: "the layer's width, as a multiplier about its own centre",
    },
    Property {
        path: path::SCALE_Y,
        describes: "the layer's height, as a multiplier about its own centre",
    },
];

/// What a layer looks like at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Properties {
    /// Offset from where the layer naturally sits, in canvas pixels.
    pub position: (f64, f64),
    /// Size multiplier about the layer's own centre. `1.0` is natural size, so
    /// scaling a layer does not also move it.
    pub scale: (f64, f64),
    /// `0.0` invisible, `1.0` solid.
    pub opacity: f64,
}

impl Default for Properties {
    /// The layer exactly as it arrived: where it sits, its own size, solid.
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            scale: (1.0, 1.0),
            opacity: 1.0,
        }
    }
}

impl Properties {
    /// Resolves a clip's animated properties at `t`, in frames from the clip's
    /// own start.
    ///
    /// Anything the clip does not animate keeps its default, so a clip with no
    /// keyframes at all composites as a plain copy.
    pub fn at(tracks: &[KeyframeTrack], t: Frames) -> Self {
        let mut properties = Self::default();
        for track in tracks {
            let Some(value) = track.value_at(t) else {
                continue;
            };
            match track.property.as_str() {
                path::OPACITY => properties.opacity = value,
                path::POSITION_X => properties.position.0 = value,
                path::POSITION_Y => properties.position.1 = value,
                path::SCALE_X => properties.scale.0 = value,
                path::SCALE_Y => properties.scale.1 = value,
                _ => {}
            }
        }
        properties
    }

    /// True when this layer would draw exactly its own pixels, unmoved and
    /// unblended — which lets a compositor copy rather than rasterise.
    pub fn is_identity(&self) -> bool {
        const EPSILON: f64 = 1e-9;
        self.position.0.abs() < EPSILON
            && self.position.1.abs() < EPSILON
            && (self.scale.0 - 1.0).abs() < EPSILON
            && (self.scale.1 - 1.0).abs() < EPSILON
            && (self.opacity - 1.0).abs() < EPSILON
    }

    /// True when the layer would contribute nothing, so it can be skipped
    /// rather than rasterised into oblivion.
    pub fn is_invisible(&self) -> bool {
        const EPSILON: f64 = 1e-9;
        self.opacity <= EPSILON || self.scale.0.abs() <= EPSILON || self.scale.1.abs() <= EPSILON
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
