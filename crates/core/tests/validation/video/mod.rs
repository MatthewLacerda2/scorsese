//! What a generated video asks for, and what the provider will not accept.
//!
//! Split by what a failure is *about*: a length another choice fixed, a still
//! that cannot be used, and a record on a kind that could never have produced
//! it.

mod bookkeeping;
mod length;
mod stills;

use crate::common::{asset_mut, project};
use scorsese_core::{Project, VideoRequest};

/// The fixture's Veo asset, with `edit` applied to its request.
pub(crate) fn asking(edit: impl FnOnce(&mut VideoRequest)) -> Project {
    let mut p = project();
    let mut request = VideoRequest::default();
    edit(&mut request);
    asset_mut(&mut p, "shot-city").video = Some(request);
    p
}
