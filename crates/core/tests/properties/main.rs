//! What the framerate and keyframe arithmetic does, over the whole of its
//! input range rather than at the values someone thought of.
//!
//! These sit **alongside** `grid/fps.rs` and `animation.rs`, which stay. An
//! example is the readable statement of what a rule *is* — a decimal
//! framerate is refused, a hold jumps on arrival — and one case states it.
//! Arithmetic is the other kind of thing: it fails at the values nobody
//! imagined, so what is asserted here is a claim of the form "for **any**
//! rate and **any** frame count, this holds".
//!
//! Every property below is a real bug if it is false, and each says which
//! bug: a cut that rewrote itself, an animation that arrived a frame early,
//! a render that crashed on a keyframe nobody looked at.
//!
//! ## Reproducible, which is the condition this target exists under
//!
//! A generated-input test that passes a hundred runs and fails the
//! hundred-and-first is worse than no test at all here, because CI's contract
//! is that red means a claim was broken. A gate that goes red for a cause
//! nobody can see teaches everyone that red means *try again*.
//!
//! So nothing in this target draws from entropy: [`runner::check`] fixes the
//! case count and the seed, and the same binary run twice examines the same
//! inputs in the same order. `tests/proptest-regressions/` is committed for
//! the same reason — a failure found once is replayed first on every later
//! run, which turns it into a permanent example test with the input written
//! down. That file is authored work and is not rebuildable, so it is
//! committed like `recipes/` and never gitignored.

mod easing;
mod fps;
mod keyframes;
mod runner;
