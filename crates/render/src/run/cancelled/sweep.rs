//! Where a cancel can land against the encoder, every place in turn (#1439).
//!
//! A web preview superseded mid-encode once showed "Broken pipe (os error 32)"
//! as a failure, and the question was whether a cancel could break the
//! encoder's pipe before the flag was read. It cannot: the flag is read
//! before each frame and a cancel never touches ffmpeg, so a render stopped
//! at any check — before the mix, between stages, at every frame — ends
//! `Cancelled`, never with a pipe error. Every check was walked once, 94
//! renders in twenty seconds; this keeps the ones that differ: each stage
//! boundary, the first and a middle frame, the last frame and past it.

use super::{LENGTH, render};
use crate::{Cancel, RenderError};

#[test]
fn a_cancel_at_any_check_ends_cancelled_never_with_a_broken_pipe() {
    let tools = crate::testing::tools();
    // Two checks before the picture, then one a frame: past the last frame it
    // finishes.
    for checks in [0, 1, 2, 3, LENGTH / 2, LENGTH + 1, LENGTH + 2, LENGTH + 3] {
        match render(&tools, Cancel::after(checks), &format!("sweep-{checks}")) {
            Ok(_) | Err(RenderError::Cancelled { .. }) => {}
            Err(other) => panic!("a cancel after {checks} checks ended {other:?}"),
        }
    }
}
