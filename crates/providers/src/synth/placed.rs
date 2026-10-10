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
//! One function per question a recipe can ask, each adding its own line: how
//! long the clip playing a song is (#1000), and where in the song the clips
//! its sections are anchored at start (#1009).

use scorsese_core::{AssetId, Clip, Project};
use scorsese_zimmer::Song;
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
    lines.extend(anchors(song, project, id)?);
    Ok(lines)
}

/// Puts a time on every anchor that names a clip: where that clip starts, in
/// seconds of the song as the clip playing it plays it — and refuses one that
/// would land before the anchor written ahead of it, naming both, since
/// those are the two clips to move.
fn anchors(
    song: &mut Song,
    project: &Project,
    id: &AssetId,
) -> Result<Vec<String>, SynthesisError> {
    let mut lines = Vec::new();
    let mut previous: Option<(String, f32)> = None;
    for anchor in &mut song.anchors {
        let (Some(clip), None) = (anchor.clip.clone(), anchor.seconds) else {
            previous = anchor
                .seconds
                .map(|seconds| (format!("{seconds} s"), seconds));
            continue;
        };
        let seconds = clip_start(project, id, &clip)?;
        if let Some((earlier, earlier_seconds)) = previous
            && seconds <= earlier_seconds
        {
            return Err(SynthesisError::AnchorsOutOfOrder {
                id: id.clone(),
                earlier,
                earlier_seconds,
                later: clip,
                later_seconds: seconds,
            });
        }
        lines.push(format!("anchor:{}:{clip}={seconds}", anchor.section));
        *anchor = anchor.resolved(seconds);
        previous = Some((format!("clip `{clip}`"), seconds));
    }
    Ok(lines)
}

/// How far into the song clip `name` starts, in seconds of the song.
///
/// Measured from the clip that plays the song, on the timeline they share — a
/// group's members are measured against a placement inside that group, since
/// their starts are counted from the group's own. The playing clip's in-point
/// and rate count, the way they do for a `fit`: a song entered two seconds in
/// and played at 2× reaches its sixth second one second after the clip starts.
///
/// Several placements are fine while they agree, and refused with each named
/// when they do not.
fn clip_start(project: &Project, id: &AssetId, name: &str) -> Result<f32, SynthesisError> {
    let refuse = |why: String| SynthesisError::AnchorClip {
        id: id.clone(),
        clip: name.to_owned(),
        why,
    };
    let lanes = timelines(project);
    let Some((lane, target)) = lanes.iter().find(|(_, clip)| clip.id.as_str() == name) else {
        return Err(refuse("names no clip in the project".into()));
    };
    let fps = project.timeline_fps;
    let starts: Vec<(&Clip, f64)> = lanes
        .iter()
        .filter(|(other, clip)| other == lane && &clip.asset == id)
        .map(|(_, playing)| {
            let after = target.start.get() as f64 - playing.start.get() as f64;
            let frames = playing.source_in.get() as f64 + after * playing.speed.get();
            (*playing, fps.seconds_at(frames))
        })
        .collect();
    let Some(&(playing, first)) = starts.first() else {
        return Err(refuse(
            "is not on a timeline any clip playing the song is on — place the song beside it"
                .into(),
        ));
    };
    if first < 0.0 {
        return Err(refuse(format!(
            "starts {:.3} s before the song does in clip `{}`",
            -first, playing.id
        )));
    }
    if starts.iter().any(|(_, seconds)| *seconds != first) {
        let clips = starts
            .iter()
            .map(|(clip, seconds)| format!("`{}` {seconds:.3} s", clip.id))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(refuse(format!(
            "falls at different points of the song in the clips playing it ({clips}) — keep \
             one placement, or give each its own recipe"
        )));
    }
    Ok(first as f32)
}

/// Every clip in the document, with the group it sits in — `None` for the
/// project's own timeline — since a start means something only against
/// another start on the same one.
fn timelines(project: &Project) -> Vec<(Option<&AssetId>, &Clip)> {
    let own = project.clips().map(|(_, clip)| (None, clip));
    let groups = project.assets.iter().flat_map(|asset| {
        let members = asset.group.iter().flat_map(|group| group.clips());
        members.map(move |(_, clip)| (Some(&asset.id), clip))
    });
    own.chain(groups).collect()
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
