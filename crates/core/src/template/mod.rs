//! Templates: a piece of an edit, saved to be copied into other edits — an
//! intro, an outro, a running gag, the whole shape of a daily video (#546).
//!
//! Two operations, and nothing else: [`extract`] lifts a set of clips out of a
//! project, with everything they need to stand alone, and [`insert`] copies
//! one into a project at a chosen time. Both live here rather than in whichever
//! client stores templates, so the hosted server, the CLI and the local MCP
//! server all get the same answer from one implementation.
//!
//! ## A template is a `project.json` document
//!
//! What [`extract`] makes is a [`Project`](crate::Project): the current
//! `schema_version`, a `name` (the template's), the source's `timeline_fps`,
//! and `assets` and `tracks` holding only what was chosen. **Not a new
//! format** — a fragment in the same shape as the document it came from, so it
//! needs no schema of its own and costs `project.json` nothing. Three things follow from that for
//! free:
//!
//! - [`Project::validate`](crate::Project::validate) is what says a template
//!   is coherent: every clip's asset is in it, no two clips share a frame of
//!   a lane.
//! - A schema bump carries templates forward with the **same**
//!   [`crate::migrate`] steps that carry projects, because a stored template is
//!   a stored document like any other.
//! - The frame rate travels with it, so a template saved on a 30 fps timeline
//!   means the same seconds on a 24 fps one — [`insert`] conforms it.
//!
//! **Times start at zero.** The earliest clip chosen opens the template, so a
//! template is "four seconds long", not "from 0:37 to 0:41 of some video";
//! keyframes are clip-relative and need nothing.
//!
//! ## Copy, never link
//!
//! Inserting copies the clips, so editing a template later never changes a
//! video it was already used in (#527). The **files** are not copied: an asset
//! carrying a `sha256` the project already has becomes a reference to that
//! asset, exactly as importing the same bytes twice does
//! ([`crate::pool::reference_asset`]). Everything else — a caption, a colour,
//! a brief — is copied under an id free in the project.
//!
//! ## Where a template's tracks land
//!
//! By **position among the tracks of the same kind**, counted from the bottom:
//! the template's first video track goes onto the project's first video track,
//! its second onto the second, and audio the same way. That is what a template
//! saved from one video means in the next one made the same way — the intro's
//! footage on the footage lane, its title on the lane above — and it is what
//! keeps a project that uses three templates at three lanes rather than nine.
//!
//! When the clips will not fit where they would land — something is already in
//! that stretch of the lane — that track and **every one after it** of the same
//! kind goes onto new tracks, appended in the template's order. The "every one
//! after it" is the point: video lanes composite in order, and sending only the
//! blocked lane to the top would draw the template's background over its own
//! title. Once spilled, a template keeps its own layering, drawn over the
//! project's. A project with no tracks yet simply gets the template's.
//!
//! An id is kept wherever it is free: a clip, an asset or a new track keeps
//! the id it had in the template, suffixed (`-2`, `-3`) only when the project
//! already uses it, and a new track takes the next `v`/`a` number instead —
//! `v1` taken is a track called `v2`, not `v1-2`. What moved where is in
//! [`Inserted`], so a caller never has to guess what it just wrote.
//!
//! **All or nothing**, as everything in [`crate::placing`] is: worked out on a
//! copy, and only a copy [`Project::validate`](crate::Project::validate)
//! accepts becomes the document.
//!
//! What is deliberately not here: inserting *with a ripple* (pushing what is
//! after the insertion point later) — that is an edit of its own, not a way of
//! inserting — and files that live only in a local project's folder, which a
//! template saved elsewhere cannot bring along; whoever stores templates for a
//! local folder decides that.

mod extract;
mod ids;
mod insert;
mod retime;

pub use extract::{ExtractError, extract};
pub use insert::{InsertError, Inserted, insert};

use crate::asset::Asset;
use crate::shape::{Attach, Endpoint, Geometry};

/// The clips an arrow asset follows — its [`Attach`]ed ends.
fn follows(asset: &Asset) -> Vec<&Attach> {
    match asset.shape.as_ref().map(|shape| &shape.geometry) {
        Some(Geometry::Arrow { from, to, .. }) => [from, to]
            .into_iter()
            .filter_map(Endpoint::attach)
            .collect(),
        _ => Vec::new(),
    }
}

/// The same, to rename what they follow.
fn follows_mut(asset: &mut Asset) -> Vec<&mut Attach> {
    match asset.shape.as_mut().map(|shape| &mut shape.geometry) {
        Some(Geometry::Arrow { from, to, .. }) => [from, to]
            .into_iter()
            .filter_map(|end| match end {
                Endpoint::Attached { attach } => Some(attach),
                Endpoint::At(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
