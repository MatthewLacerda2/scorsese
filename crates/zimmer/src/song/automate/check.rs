//! What a curve is refused for, before any samples are produced: everything
//! that would leave it moving nothing, or moving it somewhere nobody wrote.

use super::{Automation, Param, Point};
use crate::error::SynthError;
use crate::patch::Patch;
use crate::song::Song;

/// Every curve that moves a value across the piece.
///
/// A curve is refused for everything that would leave it moving *nothing*,
/// which is the same failure as a typo'd track name and gets the same
/// treatment: an automation nobody can hear is worse than one that is
/// refused, because the recipe still says the build is there.
///
/// So: a track that does not exist, no points at all, two curves on one
/// track and parameter, beats that do not ascend once placed, a beat or a
/// value that is not a number, and a point against a section the song does
/// not have or placed before the piece starts. A `cutoff` point at or below
/// zero is refused for the reason a written cutoff is — there is no filter at
/// 0 Hz. A `send` curve is refused on a song whose chain has no `reverb` or
/// `delay`, since a send reaches nothing else.
///
/// What is **not** here is the other half of the same idea: a `cutoff`
/// curve on a track whose instrument has no filter. That one needs the
/// patch, which a named reference does not carry, so it is checked in
/// [`check_resolved`] the moment the resolver has answered.
pub(in crate::song) fn check(song: &Song) -> Result<(), SynthError> {
    for (index, curve) in song.automation.iter().enumerate() {
        let (track, param) = (curve.track.clone(), curve.param.as_str());
        if !song.tracks.iter().any(|it| it.name == curve.track) {
            return Err(SynthError::UnknownAutomationTrack { track, param });
        }
        if song.automation[..index]
            .iter()
            .any(|earlier| earlier.track == curve.track && earlier.param == curve.param)
        {
            return Err(SynthError::DuplicateAutomation { track, param });
        }
        check_curve(song, curve)?;
        if curve.param == Param::Send && !crate::song::mix::has_a_room(&song.fx) {
            return Err(SynthError::AutomationWithoutRoom { track });
        }
    }
    Ok(())
}

/// One curve's points: numbers that are numbers, sections the song has, and
/// beats that ascend once placed.
fn check_curve(song: &Song, curve: &Automation) -> Result<(), SynthError> {
    let (track, param) = (curve.track.clone(), curve.param.as_str());
    let bad_curve = |why| SynthError::BadAutomationCurve {
        track: curve.track.clone(),
        param,
        why,
    };
    if curve.points.is_empty() {
        return Err(bad_curve("it has no points, so it moves nothing"));
    }
    let mut placed: Vec<(&Point, f32)> = Vec::with_capacity(curve.points.len());
    for point in &curve.points {
        let bad_point = |field, value| SynthError::BadAutomationPoint {
            track: curve.track.clone(),
            param,
            field,
            value,
        };
        let below_zero = point.section.is_none() && point.beat < 0.0;
        if !point.beat.is_finite() || below_zero {
            return Err(bad_point("beat", point.beat));
        }
        if !point.value.is_finite() {
            return Err(bad_point("value", point.value));
        }
        if curve.param == Param::Cutoff && point.value <= 0.0 {
            return Err(SynthError::BadAutomationCutoff {
                track,
                cutoff: point.value,
            });
        }
        let bad_section = |why| SynthError::BadAutomationSection {
            track: curve.track.clone(),
            param,
            point: as_written(point),
            why,
        };
        let Some(beat) = super::placed_beat(song, point) else {
            return Err(bad_section(unknown_section(song)));
        };
        if beat < 0.0 {
            return Err(bad_section(format!(
                "comes to beat {beat}, before the piece starts"
            )));
        }
        placed.push((point, beat));
    }
    // Last, so a NaN beat is reported as the number it is not rather than as
    // an ordering it happens to break — and on the placed beats, which are
    // the ones a curve is read at.
    for pair in placed.windows(2) {
        let ((before, at), (after, beat)) = (pair[0], pair[1]);
        if at < beat {
            continue;
        }
        if before.section.is_none() && after.section.is_none() {
            return Err(bad_curve("`beat` must ascend, and no two may be equal"));
        }
        return Err(SynthError::BadAutomationSection {
            track,
            param,
            point: as_written(after),
            why: format!(
                "comes to beat {beat}, not after the point at {} before it (beat {at}) — \
                 points must ascend once placed",
                as_written(before)
            ),
        });
    }
    Ok(())
}

/// A point as its author wrote it: `beat 10`, or `beat -8 of `chorus``.
fn as_written(point: &Point) -> String {
    match &point.section {
        None => format!("beat {}", point.beat),
        Some(name) => format!("beat {} of `{name}`", point.beat),
    }
}

/// Why a point's section was not found, with the sections there are.
fn unknown_section(song: &Song) -> String {
    let names: Vec<String> = song
        .arrangement
        .iter()
        .filter_map(|entry| entry.marker())
        .map(|name| format!("`{name}`"))
        .collect();
    if names.is_empty() {
        return "names a section, and the song has no sections: start one with a \
                `{ \"section\": \"<name>\" }` marker in the arrangement, or count the beat \
                from the start of the piece"
            .to_owned();
    }
    format!(
        "names a section the song does not have; its sections are {}",
        names.join(", ")
    )
}

/// What can only be checked once every track's patch is in hand.
///
/// A `cutoff` curve on an instrument with no filter has nothing to move, and a
/// curve that moves nothing is the failure this whole check exists to prevent.
/// It cannot be caught in [`Song::validate`] because a track may name its patch
/// rather than carry it, and this crate does not resolve names.
pub(in crate::song) fn check_resolved(song: &Song, patches: &[Patch]) -> Result<(), SynthError> {
    for curve in &song.automation {
        if curve.param != Param::Cutoff {
            continue;
        }
        let filtered = song
            .tracks
            .iter()
            .zip(patches)
            .find(|(track, _)| track.name == curve.track)
            .is_some_and(|(_, patch)| patch.filter.is_some());
        if !filtered {
            return Err(SynthError::AutomationWithoutFilter {
                track: curve.track.clone(),
            });
        }
    }
    Ok(())
}
