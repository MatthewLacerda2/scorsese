//! A matte that is never on screen with the clip it masks.
//!
//! A masked clip shows only through its matte, so one whose matte never shares
//! an instant with it never shows at all — a clip that renders as nothing for
//! its whole length, which looks exactly like a render that lost it. Inverted,
//! the same mistake is quieter: the matte never cuts anything out, and the
//! clip plays as though it had none.
//!
//! **A warning, not a refusal.** The document is coherent and the render is
//! correct; it is only almost certainly not what anybody meant. Which clips a
//! matte may name at all is validation's, and a matte naming nothing is
//! reported there rather than twice.

use std::collections::HashMap;

use scorsese_core::{Clip, ClipId, Project};

/// Every masked clip that never meets its matte, already worded.
pub(super) fn never_met(project: &Project) -> Vec<String> {
    let clips: HashMap<&ClipId, &Clip> = project
        .every_clip()
        .map(|(_, clip)| (&clip.id, clip))
        .collect();
    project
        .every_clip()
        .filter_map(|(_, clip)| {
            let matte = clip.matte.as_ref()?;
            let through = clips.get(&matte.clip)?;
            (!clip.overlaps(through)).then(|| worded(clip, &matte.clip, matte.invert))
        })
        .collect()
}

fn worded(clip: &Clip, matte: &ClipId, invert: bool) -> String {
    let outcome = if invert {
        "so the matte never cuts anything out of it"
    } else {
        "so it is shown through nothing and never appears"
    };
    format!(
        "clip `{}`: its matte, clip `{matte}`, is never on screen at the same time as it — \
         {outcome}; move one of the two so they overlap",
        clip.id
    )
}
