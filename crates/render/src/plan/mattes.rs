//! Taking matte clips off the stack and handing each to the shot it masks.
//!
//! A clip named as a matte is **never drawn on its own track** — being named is
//! what makes it one ([`scorsese_core::Matte`]). So it is not a layer of the
//! stack: it travels with the shot it masks, the way a group's members travel
//! with the group, and everything that reasons about the frame sees the masked
//! shot as the one thing on screen.
//!
//! A masked shot whose matte is not on screen at this instant shows through
//! nothing, and is not a layer either — or, inverted, has nothing cut out of
//! it and is an ordinary one. That is decided here rather than drawn, so a
//! render never spends a frame on a layer that cannot show.

use std::collections::HashSet;

use scorsese_core::{ClipId, Track};

use super::{Matted, Shot};

/// `shots`, one timeline's worth at one instant, with every matte clip taken
/// out and attached to the shot it masks. `scope` is that timeline's tracks,
/// which is where a matte is named from: a matte and the clip it masks share a
/// timeline, which validation guarantees.
pub(super) fn attach<'a, 't>(
    scope: impl IntoIterator<Item = &'t Track>,
    shots: Vec<Shot<'a>>,
) -> Vec<Shot<'a>> {
    let named: HashSet<&ClipId> = scope
        .into_iter()
        .flat_map(|track| &track.clips)
        .filter_map(|clip| clip.matte.as_ref().map(|matte| &matte.clip))
        .collect();
    if named.is_empty() {
        return shots;
    }
    let (mattes, shown): (Vec<_>, Vec<_>) = shots
        .into_iter()
        .partition(|shot| named.contains(&shot.clip.id));
    shown
        .into_iter()
        .filter_map(|mut shot| {
            let Some(matte) = &shot.clip.matte else {
                return Some(shot);
            };
            match mattes
                .iter()
                .find(|candidate| candidate.clip.id == matte.clip)
            {
                Some(through) => {
                    shot.matte = Some(Matted {
                        shot: Box::new(through.clone()),
                        invert: matte.invert,
                    });
                    Some(shot)
                }
                // Nothing to cut a hole in it with: shown whole.
                None if matte.invert => Some(shot),
                // Nothing to be seen through: not shown at all.
                None => None,
            }
        })
        .collect()
}
