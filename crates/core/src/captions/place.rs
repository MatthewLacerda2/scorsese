//! Writing captions into a project, and taking the last run's away first.

use std::collections::BTreeSet;
use std::path::Path;

use crate::keyframe::{Easing, Keyframe, KeyframeTrack};
use crate::text::{Reveal, RevealUnit};
use crate::timeline::{Anchor, AnchorY, Clip, ClipId, Track, TrackId, TrackKind};
use crate::words::Words;
use crate::{Asset, AssetId, AssetKind, Frames, Project};

use super::{Captioning, Chunk, PREFIX, TOOL, chunks, time};

/// What one run of captioning did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Captioned {
    /// Each captioned narration clip, with how many captions it got.
    pub lines: Vec<(ClipId, usize)>,
    /// Narration clips with no word timings, skipped.
    pub untimed: Vec<ClipId>,
    /// How many captions from an earlier run were taken away first.
    pub replaced: usize,
}

/// Why a run wrote nothing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CaptionError {
    /// The track named for the captions carries sound.
    #[error("`{0}` is an audio track — captions go on a video track")]
    NotVideo(TrackId),
    /// A caption would sit over a clip that is not one, on the captions' track.
    #[error(
        "caption `{caption}` would overlap `{clip}` on track `{track}` — name a \
         track of their own for the captions"
    )]
    Overlap {
        /// The caption.
        caption: ClipId,
        /// The clip already there.
        clip: ClipId,
        /// The track.
        track: TrackId,
    },
    /// An id a caption needs is held by something the run did not write.
    #[error("`{0}` is already taken by something that is not a caption")]
    Taken(String),
}

/// Writes captions for every timed narration line in `project`, read from the
/// timings beside their audio under `root`.
///
/// Nothing is changed when it fails: the run is worked on a copy and kept
/// whole or not at all.
pub fn caption(
    project: &mut Project,
    root: &Path,
    asked: &Captioning,
) -> Result<Captioned, CaptionError> {
    let mut next = project.clone();
    let mut report = Captioned {
        replaced: withdraw(&mut next),
        ..Captioned::default()
    };
    let mut all = Vec::new();
    for clip in narration(&next, asked) {
        let words = next
            .asset(&clip.asset)
            .and_then(|asset| Words::of(asset, root));
        match words {
            Some(words) => {
                let placed = words.placed(clip, next.timeline_fps);
                let cut = chunks(&clip.id, &placed, asked.chunking);
                report.lines.push((clip.id.clone(), cut.len()));
                all.extend(cut);
            }
            None => report.untimed.push(clip.id.clone()),
        }
    }
    place(&mut next, time(all, asked.chunking), asked)?;
    *project = next;
    Ok(report)
}

/// The narration clips `asked` names, in document order.
fn narration<'a>(project: &'a Project, asked: &Captioning) -> Vec<&'a Clip> {
    project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .filter(|track| asked.narration.is_empty() || asked.narration.contains(&track.id))
        .flat_map(|track| &track.clips)
        .filter(|clip| {
            !asked.narration.is_empty()
                || project
                    .asset(&clip.asset)
                    .is_some_and(|asset| asset.kind == AssetKind::GeneratedAudio)
        })
        .collect()
}

/// Takes away every caption an earlier run wrote, and the caption assets
/// nothing shows any more. Returns how many clips went.
fn withdraw(project: &mut Project) -> usize {
    let mut gone = 0;
    for track in &mut project.tracks {
        let before = track.clips.len();
        track
            .clips
            .retain(|clip| !(is_ours(clip.id.as_str()) && is_ours(clip.asset.as_str())));
        gone += before - track.clips.len();
    }
    let shown: BTreeSet<AssetId> = project
        .every_clip()
        .map(|(_, clip)| clip.asset.clone())
        .collect();
    project.assets.retain(|asset| {
        !(asset.kind == AssetKind::Text && is_ours(asset.id.as_str()) && !shown.contains(&asset.id))
    });
    gone
}

fn is_ours(id: &str) -> bool {
    id.starts_with(PREFIX)
}

/// Puts each caption on the captions' track, making the track if need be.
fn place(project: &mut Project, timed: Vec<Chunk>, asked: &Captioning) -> Result<(), CaptionError> {
    let fps = project.timeline_fps;
    let index = match project.tracks.iter().position(|t| t.id == asked.track) {
        Some(index) if project.tracks[index].kind != TrackKind::Video => {
            return Err(CaptionError::NotVideo(asked.track.clone()));
        }
        Some(index) => index,
        None => {
            project
                .tracks
                .push(Track::new(asked.track.clone(), TrackKind::Video));
            project.tracks.len() - 1
        }
    };
    let mut counts: Vec<(ClipId, usize)> = Vec::new();
    for chunk in timed {
        let start = fps.frames(chunk.start);
        let end = fps.frames(chunk.end);
        if end <= start {
            continue;
        }
        let n = match counts.iter_mut().find(|(line, _)| line == &chunk.line) {
            Some((_, n)) => {
                *n += 1;
                *n
            }
            None => {
                counts.push((chunk.line.clone(), 1));
                1
            }
        };
        let id = format!("{PREFIX}{}-{n}", chunk.line);
        let taken = project.asset(&AssetId::new(id.as_str())).is_some()
            || project.every_clip().any(|(_, clip)| clip.id.as_str() == id);
        if taken {
            return Err(CaptionError::Taken(id));
        }
        let clip = clip(&id, start, Frames(end.get() - start.get()), asked);
        let track = &mut project.tracks[index];
        if let Some(other) = track.clips.iter().find(|other| other.overlaps(&clip)) {
            return Err(CaptionError::Overlap {
                caption: clip.id,
                clip: other.id.clone(),
                track: track.id.clone(),
            });
        }
        track.clips.push(clip);
        project.assets.push(asset(&id, &chunk, asked));
    }
    project.tracks[index].clips.sort_by_key(|clip| clip.start);
    Ok(())
}

/// The text a caption shows, in the run's style, arriving whole.
fn asset(id: &str, chunk: &Chunk, asked: &Captioning) -> Asset {
    let mut asset = Asset::text(AssetId::new(id), chunk.text.as_str());
    let mut style = asked.style.clone();
    style.reveal = Some(Reveal {
        unit: RevealUnit::Word,
        stagger: 0.0,
        ..Reveal::default()
    });
    asset.style = Some(style);
    asset.note = Some(format!(
        "A caption of what `{}` says, written by the captioning tool. Running \
         it again rewrites this caption; rename it to keep an edit.",
        chunk.line
    ));
    asset
}

/// The clip that shows it, resting on the frame's bottom edge and lifted.
fn clip(id: &str, start: Frames, duration: Frames, asked: &Captioning) -> Clip {
    let mut clip = Clip::new(ClipId::new(id), AssetId::new(id), start, duration);
    clip.anchor = Anchor {
        y: AnchorY::Bottom,
        ..Anchor::default()
    };
    let point = |t: u64, value: f64, easing| Keyframe {
        t: Frames(t),
        value,
        easing,
    };
    clip.keyframes.push(
        KeyframeTrack::new(
            asked.height.clone(),
            vec![point(0, asked.lift, Easing::Linear)],
        )
        .generated_by(TOOL),
    );
    let arrive = asked.arrive.min(duration.get().saturating_sub(1));
    if arrive > 0 {
        clip.keyframes.push(
            KeyframeTrack::new(
                asked.reveal.clone(),
                vec![
                    point(0, 0.0, Easing::EaseOut),
                    point(arrive, 1.0, Easing::Linear),
                ],
            )
            .generated_by(TOOL),
        );
    }
    clip
}
