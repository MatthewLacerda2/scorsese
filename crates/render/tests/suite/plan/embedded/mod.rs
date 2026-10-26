//! Which clips on a *video* track are also heard.
//!
//! Two questions, one per file: whether a shot contributes sound at all
//! ([`audibility`]), and what its being heard does to where the mix is cut
//! ([`boundaries`]).

mod audibility;
mod boundaries;

use scorsese_core::{Fps, Project};
use scorsese_render::{FrameRange, Plan};

fn plan_of(project: &Project) -> Plan<'_> {
    Plan::build(project, Fps::THIRTY, FrameRange::ALL).expect("plan")
}
