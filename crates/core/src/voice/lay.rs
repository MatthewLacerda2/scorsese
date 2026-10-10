//! The arithmetic: where every named clip goes.

use std::collections::BTreeSet;
use std::path::Path;

use super::{Laid, Measured, Scene, VoiceError, Voicing};
use crate::project::Project;
use crate::time::{Fps, Frames};
use crate::timeline::{Clip, ClipId, TrackKind};
use crate::words::Words;

/// One clip's new place in time.
#[derive(Debug, Clone)]
pub(super) struct Move {
    pub(super) clip: ClipId,
    pub(super) start: Frames,
    pub(super) duration: Frames,
}

/// The whole layout, before it touches the document.
#[derive(Debug)]
pub(super) struct Plan {
    /// In scene order: each scene's line, then its visuals, then its riders.
    pub(super) moves: Vec<Move>,
    pub(super) laid: Vec<Laid>,
    /// Each scene's span before — earliest visual start to latest visual end.
    pub(super) before: Vec<(Frames, Frames)>,
    /// Each scene's span after, its visuals' overlap included.
    pub(super) after: Vec<(Frames, Frames)>,
    pub(super) was: Frames,
}

/// Refuses a call that names something impossible, before any arithmetic.
pub(super) fn check(project: &Project, voicing: &Voicing) -> Result<(), VoiceError> {
    if voicing.scenes.is_empty() {
        return Err(VoiceError::NoScenes);
    }
    let seconds = [
        ("lead_in", Some(voicing.lead_in)),
        ("gap", Some(voicing.gap)),
        ("overlap", Some(voicing.overlap)),
        ("from", voicing.from),
    ];
    let leads = voicing
        .scenes
        .iter()
        .map(|scene| ("lead_in", scene.lead_in));
    for (key, value) in seconds.into_iter().chain(leads) {
        if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
            return Err(VoiceError::Seconds(key));
        }
    }
    let mut named = BTreeSet::new();
    for scene in &voicing.scenes {
        if scene.visuals.is_empty() {
            return Err(VoiceError::NoVisual(scene.line.clone()));
        }
        let all = std::iter::once((&scene.line, Some(TrackKind::Audio)))
            .chain(scene.visuals.iter().map(|id| (id, Some(TrackKind::Video))))
            .chain(scene.riders.iter().map(|id| (id, None)));
        for (id, kind) in all {
            if !named.insert(id.clone()) {
                return Err(VoiceError::Twice(id.clone()));
            }
            let (track, _) = find(project, id)?;
            match kind {
                Some(TrackKind::Audio) if track != TrackKind::Audio => {
                    return Err(VoiceError::NotALine(id.clone()));
                }
                Some(TrackKind::Video) if track != TrackKind::Video => {
                    return Err(VoiceError::NotAVisual(id.clone()));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Works out every scene's span and every named clip's new place.
pub(super) fn plan(project: &Project, root: &Path, voicing: &Voicing) -> Result<Plan, VoiceError> {
    let fps = project.timeline_fps;
    let before: Vec<(Frames, Frames)> = voicing
        .scenes
        .iter()
        .map(|scene| span(project, scene))
        .collect::<Result<_, _>>()?;
    let was = before
        .iter()
        .map(|(_, end)| *end)
        .max()
        .unwrap_or(Frames::ZERO);
    let mut cursor = voicing.from.map_or(before[0].0, |from| fps.frames(from));
    let mut plan = Plan {
        moves: Vec::new(),
        laid: Vec::new(),
        before: before.clone(),
        after: Vec::new(),
        was,
    };
    let last = voicing.scenes.len() - 1;
    for (index, scene) in voicing.scenes.iter().enumerate() {
        let start = cursor;
        let lead = fps.frames(scene.lead_in.unwrap_or(voicing.lead_in));
        let (_, line) = find(project, &scene.line)?;
        let mut line = line.clone();
        line.start = start + lead;
        let (spoken, measured) = spoken(project, root, &line);
        let end = fps
            .frames(spoken + voicing.gap)
            .max(Frames(start.get() + 1));
        let runs_on = if index == last {
            Frames::ZERO
        } else {
            fps.frames(voicing.overlap)
        };
        let shown = Frames(end.get() - start.get()) + runs_on;

        plan.moves.push(Move {
            clip: line.id.clone(),
            start: line.start,
            duration: line.duration,
        });
        for id in &scene.visuals {
            plan.moves.push(Move {
                clip: id.clone(),
                start,
                duration: shown,
            });
        }
        let shift = start.get() as i128 - before[index].0.get() as i128;
        for id in &scene.riders {
            let (_, rider) = find(project, id)?;
            let moved = u64::try_from(rider.start.get() as i128 + shift)
                .map_err(|_| VoiceError::BeforeTheStart(id.clone()))?;
            plan.moves.push(Move {
                clip: id.clone(),
                start: Frames(moved),
                duration: rider.duration,
            });
        }
        plan.after.push((start, start + shown));
        plan.laid.push(Laid {
            line: scene.line.clone(),
            start,
            end,
            measured,
        });
        cursor = end;
    }
    trim_tails(project, &mut plan.moves, &voicing.scenes);
    Ok(plan)
}

/// When a line's last word is said, in seconds of the timeline, with the line
/// at its new place — or its clip's end, when it has no word timings.
fn spoken(project: &Project, root: &Path, line: &Clip) -> (f64, Measured) {
    let fps: Fps = project.timeline_fps;
    let last = project
        .asset(&line.asset)
        .and_then(|asset| Words::of(asset, root))
        .and_then(|words| {
            let placed = words.placed(line, fps);
            placed.iter().map(|word| word.end).reduce(f64::max)
        });
    match last {
        Some(seconds) => (seconds, Measured::LastWord),
        None => (fps.seconds(line.end()), Measured::EndOfAudio),
    }
}

/// Shortens a line whose trailing silence would now run into the next line on
/// the same track. The cut lands after its last word and the gap, so nothing
/// said is lost: the next line starts no earlier than that.
fn trim_tails(project: &Project, moves: &mut [Move], scenes: &[Scene]) {
    let track_of = |id: &ClipId| {
        project
            .clips()
            .find(|(_, clip)| &clip.id == id)
            .map(|(track, _)| track.id.clone())
    };
    let starts: Vec<(ClipId, Frames)> = moves.iter().map(|m| (m.clip.clone(), m.start)).collect();
    for pair in scenes.windows(2) {
        if track_of(&pair[0].line) != track_of(&pair[1].line) {
            continue;
        }
        let next = starts.iter().find(|(id, _)| id == &pair[1].line);
        let line = moves.iter_mut().find(|m| m.clip == pair[0].line);
        if let (Some((_, next)), Some(line)) = (next, line)
            && line.start + line.duration > *next
        {
            line.duration = Frames(next.get() - line.start.get());
        }
    }
}

/// A scene's span before: its earliest visual's start to its latest's end.
fn span(project: &Project, scene: &Scene) -> Result<(Frames, Frames), VoiceError> {
    let mut start = None::<Frames>;
    let mut end = Frames::ZERO;
    for id in &scene.visuals {
        let (_, clip) = find(project, id)?;
        start = Some(start.map_or(clip.start, |s| s.min(clip.start)));
        end = end.max(clip.end());
    }
    let start = start.ok_or_else(|| VoiceError::NoVisual(scene.line.clone()))?;
    Ok((start, end))
}

/// The clip with this id, and the kind of track holding it.
pub(super) fn find<'a>(
    project: &'a Project,
    id: &ClipId,
) -> Result<(TrackKind, &'a Clip), VoiceError> {
    project
        .clips()
        .find(|(_, clip)| &clip.id == id)
        .map(|(track, clip)| (track.kind, clip))
        .ok_or_else(|| VoiceError::NoSuchClip(id.clone()))
}
