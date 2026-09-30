//! Turning clips already on the timeline into a group, and a group back into
//! clips.
//!
//! **Grouping is the operation an agent actually wants.** A diagram is built
//! clip by clip on the timeline, where every box and arrow can be placed and
//! checked; the moment it has to move as one, the clips are *wrapped*: moved
//! into a new `group` asset, and replaced by one clip of it at the same place
//! and time. Building an empty group and filling it would be a second way to
//! author the same thing, so it is not here — a hand-written `group` block in
//! the document does that, for whoever wants it.
//!
//! - [`group`] wraps clips into a new group, in one edit.
//! - [`ungroup`] is its inverse: the members go back onto the timeline, on
//!   their own tracks, exactly where the group clip was showing them.
//!
//! **Both are all-or-nothing**, the way [`crate::placing`] is: worked out on a
//! copy, and only a copy [`Project::validate`](crate::Project::validate)
//! accepts becomes the document. So an arrow outside the selection attached to
//! a clip inside it — which would cross the group's edge — refuses the whole
//! request and says which arrow, rather than leaving it pointing at nothing.
//!
//! **Neither edits a member.** Moving a clip into a group changes its `start`
//! (a group's time runs from its own zero) and nothing else; its keyframes are
//! clip-relative already, and its id is kept, so everything that named it
//! still does.

mod unwrap;
mod wrap;

pub use unwrap::{UngroupError, Ungrouped, ungroup};
pub use wrap::{GroupError, Grouped, Grouping, group};

use crate::project::Project;
use crate::timeline::Track;

/// Every track in the document — the timeline's and every group's — because
/// a group's tracks come back onto the timeline when it is ungrouped, and two
/// lanes answering to one name are refused wherever they are.
fn every_track(project: &Project) -> impl Iterator<Item = &Track> {
    let groups = project
        .assets
        .iter()
        .filter_map(|asset| asset.group.as_ref())
        .flat_map(|group| &group.tracks);
    project.tracks.iter().chain(groups)
}

/// How many ids a search for a free one could meet — every clip, track and
/// asset in the document — which bounds [`crate::template::ids::free`].
fn id_limit(project: &Project) -> usize {
    project.every_clip().count() + project.assets.len() + every_track(project).count()
}
