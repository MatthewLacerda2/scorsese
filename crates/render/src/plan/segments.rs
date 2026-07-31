//! Cutting the timeline where the set of visible clips changes.
//!
//! With one video track a segment was simply a clip or a hole. With several,
//! the useful unit is a stretch over which **nothing changes**: the same clips
//! are visible on the same tracks throughout, so one decoder per layer covers
//! the whole stretch and the compositor is handed the same shaped stack every
//! frame.
//!
//! Those stretches fall out of the clip boundaries. Every clip start and every
//! clip end, on any track, is a point where the visible set can change; between
//! two consecutive such points it cannot.
//!
//! Audio is cut exactly the same way, by the same code. A stack of sounds
//! playing at once is the same problem as a stack of pictures — several tracks,
//! no overlap within a track, cuts wherever the set changes — so it would take
//! more code, not less, to treat them as two problems.
//!
//! The one difference is *which* clips take part, and neither pass is simply
//! "its own tracks". A clip on a video track is always visible; a clip on an
//! *audio* track is visible too when it is a prompt with no media, because its
//! slug card is picture. A clip on an audio track is always audible; a clip on
//! a video track is audible when its own file has sound on it. So both passes
//! take a predicate, and each one says no to something.

use scorsese_core::{Asset, Clip, Frames, Project, Track, TrackKind};

use super::{PlanError, Segment, Shot, Showing};

/// True when this clip puts something on screen.
///
/// Every clip on a video track does. A clip on an **audio** track does when it
/// is showing a slug card: a narration prompt that has not been generated is
/// silence in the mix and a card on the picture, which is the only way a cut
/// built around a voice-over can be watched before the voice-over exists. Once
/// it *has* been generated the card goes away and the picture is untouched —
/// nothing is ever drawn over a shot for a sound that can be heard.
pub(super) fn is_visible(project: &Project, track: &Track, clip: &Clip) -> bool {
    match track.kind {
        TrackKind::Video => true,
        TrackKind::Audio => project
            .asset(&clip.asset)
            .is_some_and(|asset| showing(asset) == Showing::Card),
    }
}

/// True when this clip puts sound into the mix.
///
/// A clip on an audio track always does. A clip on a **video** track does when
/// its own file carries an audio stream — the dialogue on an interview, the
/// click track on a screen recording — which is a fact about the media, read
/// from the assets table.
///
/// An asset nobody probed reads as **not audible**, deliberately. The render
/// probes what it does not know before it gets here, so by this point a missing
/// answer means the probe failed, and treating "we could not find out" as sound
/// would mean decoding a file we have no reason to believe has any. That is the
/// case a render note exists for.
pub(super) fn is_audible(project: &Project, track: &Track, clip: &Clip) -> bool {
    match track.kind {
        TrackKind::Audio => true,
        TrackKind::Video => project.asset(&clip.asset).is_some_and(carries_sound),
    }
}

/// True when the media behind this asset is known to have an audio stream.
pub(super) fn carries_sound(asset: &Asset) -> bool {
    asset
        .media
        .and_then(|media| media.audio_channels)
        .is_some_and(|channels| channels > 0)
}

/// Every track of one kind carrying clips, in project order — for video,
/// **first at the bottom**.
///
/// Audio tracks have no equivalent of that order: sounds playing at once are
/// summed, and addition does not care which came first. The order is kept
/// anyway so a mix is reproducible down to its floating-point rounding.
pub(super) fn tracks_of(project: &Project, kind: TrackKind) -> Vec<&Track> {
    project
        .tracks
        .iter()
        .filter(|track| track.kind == kind && !track.clips.is_empty())
        .collect()
}

/// The frame just past the last clip on any of these tracks.
pub(super) fn timeline_end(tracks: &[&Track]) -> Frames {
    tracks
        .iter()
        .flat_map(|track| track.clips.iter().map(Clip::end))
        .max()
        .unwrap_or(Frames::ZERO)
}

/// The tracks taking part in a pass: those with at least one clip `keep` wants.
///
/// For picture that is every video track with clips. For sound it is every
/// audio track, plus any video track carrying a clip with sound on it — sound
/// is not a property of a track's kind, it is a property of the media.
pub(super) fn taking_part<'a>(
    project: &'a Project,
    kinds: &[TrackKind],
    keep: impl Fn(&Track, &Clip) -> bool,
) -> Vec<&'a Track> {
    project
        .tracks
        .iter()
        .filter(|track| {
            kinds.contains(&track.kind) && track.clips.iter().any(|clip| keep(track, clip))
        })
        .collect()
}

/// Splits `start..end` at every clip boundary and gathers what is visible — or
/// audible — through each piece.
///
/// `keep` decides which clips take part at all: one it rejects neither appears
/// as a layer nor cuts the timeline, so a silent shot in the middle of a music
/// bed does not split the mix in two for no reason.
pub(super) fn build<'a>(
    project: &'a Project,
    tracks: &[&'a Track],
    start: Frames,
    end: Frames,
    keep: impl Fn(&Track, &Clip) -> bool,
) -> Result<Vec<Segment<'a>>, PlanError> {
    // No tracks is not one empty stretch, it is no stretches at all. The
    // difference matters for audio, where it is what tells a render to produce
    // no soundtrack rather than a silent one.
    if tracks.is_empty() {
        return Ok(Vec::new());
    }
    let mut segments = Vec::new();
    for pair in cuts(tracks, start, end, &keep).windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let mut layers = Vec::new();
        for track in tracks {
            // A track's clips never overlap — validation guarantees it — so at
            // most one is visible here. A track with a hole contributes **no
            // layer**, rather than a black one that would paint over the tracks
            // below it.
            if let Some(clip) = track
                .clips
                .iter()
                .find(|clip| covers(clip, from) && keep(track, clip))
            {
                let asset = renderable_asset(project, clip)?;
                layers.push(Shot {
                    track: &track.id,
                    clip,
                    asset,
                    showing: showing(asset),
                    source_in: source_in_at(clip, from),
                });
            }
        }
        segments.push(Segment {
            start: from,
            duration: Frames(to.get() - from.get()),
            layers,
        });
    }
    Ok(segments)
}

/// Where in a clip's source the stretch beginning at `at` opens, in frames of
/// the timeline grid.
///
/// The one piece of arithmetic a clip's speed changes. Without it a segment
/// resuming five timeline frames into a clip opened five source frames in;
/// with it the source has advanced by five *times the clip's speed*, so the
/// answer is generally not a whole frame and is not rounded to one.
///
/// An unusable speed cannot get here through a loaded project — validation
/// refuses it — and an in-memory one built by hand produces a position a
/// decoder will simply seek to, which is no worse than any other nonsense
/// number in a document nobody validated.
fn source_in_at(clip: &Clip, at: Frames) -> f64 {
    let elapsed = Frames(at.get().saturating_sub(clip.start.get()));
    clip.source_in.get() as f64 + clip.speed.source_frames(elapsed)
}

/// The frames at which the visible set can change, in order, `start` and `end`
/// included.
fn cuts(
    tracks: &[&Track],
    start: Frames,
    end: Frames,
    keep: &impl Fn(&Track, &Clip) -> bool,
) -> Vec<Frames> {
    let mut cuts = vec![start, end];
    let inside = |at: Frames| at > start && at < end;
    for track in tracks {
        for clip in track.clips.iter().filter(|clip| keep(track, clip)) {
            for boundary in [clip.start, clip.end()] {
                if inside(boundary) {
                    cuts.push(boundary);
                }
            }
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

fn covers(clip: &Clip, at: Frames) -> bool {
    clip.start <= at && at < clip.end()
}

/// Media, or the card that stands in for it.
///
/// The question is not which lifecycle state the asset is in but whether the
/// document believes there is a file: `sketch` has never had one, `queued` does
/// not have one yet, and `stale` has one that no longer answers the prompt
/// beside it. All three are a card, and `generated` is the only state that is
/// not. An asset with no lifecycle at all — an imported clip, a title — always
/// shows itself.
pub(super) fn showing(asset: &Asset) -> Showing {
    if asset.kind.is_generated() && !asset.has_renderable_media() {
        Showing::Card
    } else {
        Showing::Media
    }
}

/// The asset a clip shows, if there is anything to put on screen for it.
fn renderable_asset<'a>(project: &'a Project, clip: &Clip) -> Result<&'a Asset, PlanError> {
    let asset = project
        .asset(&clip.asset)
        .ok_or_else(|| PlanError::UnknownAsset {
            clip: clip.id.to_string(),
            asset: clip.asset.to_string(),
        })?;
    // Two kinds need no file. A text asset carries its content in the document,
    // and a prompt with nothing generated has a slug card — so the
    // missing-media refusal is only for an imported asset whose path went
    // missing from the table, which is a document nobody can render.
    if asset.kind.is_file_backed() && asset.path.is_none() && showing(asset) == Showing::Media {
        return Err(PlanError::NoMedia {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    Ok(asset)
}
