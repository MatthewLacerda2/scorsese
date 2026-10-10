//! Where a recipe meets the project: what a song leaves for the edit to say.
//!
//! The synthesiser has no I/O and has never heard of a clip, so a recipe that
//! says `"fit": { "to": "clip" }` (#1000) cannot be rendered as it stands —
//! the length it asks for is in `project.json`, on the timeline, and it moves
//! whenever the cut does. This module reads it there and writes it into the
//! song as seconds, so `zimmer` is handed exactly what it was always handed.
//!
//! **What the project said is part of what the bake renders**, so each fact
//! read here comes back as a line for the bake's address (`address`), the way
//! a named patch file's digest does. A clip made shorter is then a different
//! address — a cache miss and a re-bake, exactly as an edited recipe is — and
//! a recipe that asks the project nothing gains no line and keeps the address
//! it always had.
//!
//! One function per question a recipe can ask, each adding its own line, is
//! the shape a later question (#1009's section anchors) extends.

use scorsese_core::{AssetId, Project};
use scorsese_zimmer::song::FitTo;

use super::error::SynthesisError;
use super::recipe::Recipe;

/// Fills in everything `recipe` leaves to the project that plays asset `id`,
/// and returns what was read, one labelled line per fact, for the address.
///
/// A recipe that asks nothing is returned untouched with no lines. A `fit`
/// writing both `seconds` and `to` is left alone too: the synthesiser refuses
/// it by name, which is a better answer than this module silently picking one.
pub(super) fn resolve(
    recipe: &mut Recipe,
    project: &Project,
    id: &AssetId,
) -> Result<Vec<String>, SynthesisError> {
    let Recipe::Song(song) = recipe else {
        return Ok(Vec::new());
    };
    let mut lines = Vec::new();
    if let Some(fit) = song.fit
        && fit.to == Some(FitTo::Clip)
        && fit.seconds.is_none()
    {
        let seconds = clip_length(project, id)?;
        song.fit = Some(fit.resolved(seconds));
        lines.push(format!("fit.to:clip={seconds}"));
    }
    Ok(lines)
}

/// How long the clip that plays `id` needs the song to be, in seconds.
///
/// That is **the end of the source the clip reaches**, not only its length on
/// the timeline: a clip that starts its music two seconds in, or plays it at
/// another speed, still needs the song's last note on its own last frame, so
/// the in-point and the rate count — the same arithmetic that decides whether
/// any clip runs off the end of its media.
///
/// Every clip in the document is asked, a group's members included, since a
/// score can sit inside a group. One placement is the common case; several of
/// one length are fine; several of different lengths have no single answer and
/// are refused with each clip named, rather than fitted to whichever came
/// first.
fn clip_length(project: &Project, id: &AssetId) -> Result<f32, SynthesisError> {
    let fps = project.timeline_fps;
    let clips: Vec<(String, f64)> = project
        .every_clip()
        .filter(|(_, clip)| &clip.asset == id)
        .map(|(_, clip)| {
            let reaches = clip.source_in.get() as f64 + clip.source_frames();
            (clip.id.to_string(), fps.seconds_at(reaches))
        })
        .collect();
    let Some(&(_, first)) = clips.first() else {
        return Err(SynthesisError::NotPlaced { id: id.clone() });
    };
    if clips.iter().any(|(_, seconds)| *seconds != first) {
        let clips = clips
            .iter()
            .map(|(clip, seconds)| format!("`{clip}` {seconds:.2} s"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(SynthesisError::ClipsDisagree {
            id: id.clone(),
            clips,
        });
    }
    Ok(first as f32)
}
