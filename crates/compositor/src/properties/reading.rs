//! What a resolved instant of a layer amounts to: the scale it is really
//! drawn at, and whether it can be copied through or skipped altogether.
//!
//! Beside the resolution in `mod.rs` rather than inside it, because these are
//! questions a compositor asks of an instant once it has one, and each of them
//! has to know about every property a layer can carry — which is exactly the
//! list that grows. Keeping them together means a new property has one place
//! to say whether it stops a layer being a plain copy.

use scorsese_core::Blend;

use super::Properties;

impl Properties {
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
            // And a keyed layer is not its own pixels in the one way none of
            // the lines above would catch: the key writes *alpha*, so a layer
            // carrying nothing but a key satisfies every other condition here
            // and the copy path would hand the screen straight through, fully
            // opaque, with the key silently doing nothing at all.
            && self.chroma_key.is_none()
            // And a taped layer is not its own pixels, for the same reason
            // again: a plate carrying nothing but a `vhs` satisfies every other
            // line here, so leaving it out would copy the source through and
            // render the whole look away on exactly the clips it costs least to
            // apply to.
            && self.vhs.is_none()
            // A graded layer is not its own pixels, which is the whole point of
            // grading it. Left out, the copy path below would hand the ungraded
            // source straight to the canvas and the grade would silently do
            // nothing on exactly the clips it costs least to apply to.
            && self.grade.is_neutral()
            // And a layer with light of its own is not its own pixels: a
            // shadow or a glow reaches past them, and any blend but `normal`
            // reads what is already on the canvas — an opaque full-frame plate
            // set to `add` copied through would replace what it was meant to
            // brighten.
            && self.shadow.is_none()
            && self.glow.is_none()
            && self.blend == Blend::Normal
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
