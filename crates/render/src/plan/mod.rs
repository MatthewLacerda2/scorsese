//! Turning a timeline into an ordered list of things to decode.
//!
//! This is the part of rendering that involves no ffmpeg and no bytes: read
//! the project, work out what occupies every frame of the requested range, and
//! say how many output frames each stretch is worth. Keeping it separate is
//! what lets the sequencing be tested exhaustively without encoding anything.

mod range;

use scorsese_core::{Asset, AssetKind, Clip, Fps, Frames, Project, Track, TrackKind};

use crate::report::Note;

pub use range::{FrameRange, FrameRangeError};

/// What occupies one uninterrupted stretch of the timeline.
#[derive(Debug, Clone, PartialEq)]
pub enum Content<'a> {
    /// Nothing is on the timeline here. Gaps render black rather than being
    /// skipped: a hole in the middle of an edit is two seconds of black, not
    /// two seconds of missing time.
    Gap,
    /// One clip, showing one asset.
    Shot(Shot<'a>),
}

/// A clip resolved to the asset it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot<'a> {
    pub clip: &'a Clip,
    pub asset: &'a Asset,
    /// Where in the source to start, in frames of the **timeline** grid —
    /// the clip's own `source_in` plus however much of the clip a partial
    /// render skipped past.
    pub source_in: Frames,
}

/// One stretch of the timeline and what fills it.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment<'a> {
    /// Where this stretch begins, in timeline frames.
    pub start: Frames,
    /// How many timeline frames it covers.
    pub duration: Frames,
    pub content: Content<'a>,
}

impl Segment<'_> {
    /// The frame just past this stretch's last one.
    pub fn end(&self) -> Frames {
        self.start + self.duration
    }
}

/// The whole render, sequenced but not yet started.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan<'a> {
    start: Frames,
    end: Frames,
    timeline_fps: Fps,
    out_fps: Fps,
    segments: Vec<Segment<'a>>,
    notes: Vec<Note>,
}

impl<'a> Plan<'a> {
    /// Sequences a project's video track over `range`, for an output at
    /// `out_fps`.
    pub fn build(project: &'a Project, out_fps: Fps, range: FrameRange) -> Result<Self, PlanError> {
        let track = sole_video_track(project)?;
        let timeline_end = track
            .clips
            .iter()
            .map(Clip::end)
            .max()
            .unwrap_or(Frames::ZERO);

        let mut notes = audio_note(project);
        let start = range.start();
        let end = match range.end() {
            Some(asked) if asked > timeline_end => {
                notes.push(Note::RangeClamped {
                    asked,
                    timeline_end,
                });
                timeline_end
            }
            Some(asked) => asked,
            None => timeline_end,
        };
        if start >= end {
            return Err(PlanError::EmptyRange {
                range,
                timeline_end,
            });
        }

        let mut clips: Vec<&Clip> = track.clips.iter().collect();
        clips.sort_by_key(|clip| clip.start);

        let mut segments = Vec::new();
        let mut cursor = start;
        for clip in clips {
            let visible_start = clip.start.max(start);
            let visible_end = clip.end().min(end);
            if visible_end <= visible_start {
                continue;
            }
            if visible_start > cursor {
                segments.push(gap(cursor, visible_start));
            }
            segments.push(Segment {
                start: visible_start,
                duration: span(visible_start, visible_end),
                content: Content::Shot(Shot {
                    clip,
                    asset: renderable_asset(project, clip)?,
                    source_in: clip.source_in + span(clip.start, visible_start),
                }),
            });
            cursor = visible_end;
        }
        if cursor < end {
            segments.push(gap(cursor, end));
        }

        Ok(Self {
            start,
            end,
            timeline_fps: project.timeline_fps,
            out_fps,
            segments,
            notes,
        })
    }

    pub fn segments(&self) -> &[Segment<'a>] {
        &self.segments
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub const fn out_fps(&self) -> Fps {
        self.out_fps
    }

    /// How many frames the render writes in total.
    pub fn out_frames(&self) -> u64 {
        self.out_index(self.end)
    }

    /// How many output frames one segment is worth.
    ///
    /// Derived from the segment's timeline **boundaries** rather than from its
    /// duration, so segment counts always sum to the render's total. Conforming
    /// each duration on its own would round each one independently and drift.
    pub fn out_frames_of(&self, segment: &Segment<'_>) -> u64 {
        self.out_index(segment.end()) - self.out_index(segment.start)
    }

    /// The output frame a timeline frame lands on, counted from the start of
    /// the render.
    fn out_index(&self, at: Frames) -> u64 {
        let offset = span(self.start, at);
        self.out_fps.conform(offset, self.timeline_fps).get()
    }
}

fn gap(from: Frames, to: Frames) -> Segment<'static> {
    Segment {
        start: from,
        duration: span(from, to),
        content: Content::Gap,
    }
}

/// Frames from `from` up to `to`, floored at zero.
fn span(from: Frames, to: Frames) -> Frames {
    Frames(to.get().saturating_sub(from.get()))
}

/// The one video track carrying clips.
///
/// Compositing several video tracks together is Compositor v1's job, so more
/// than one is refused rather than silently flattened — a render that quietly
/// drops a track is worse than one that will not start. Empty video tracks
/// are ignored, since they cost nothing to allow.
fn sole_video_track(project: &Project) -> Result<&Track, PlanError> {
    let tracks: Vec<&Track> = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video && !track.clips.is_empty())
        .collect();
    match tracks.as_slice() {
        [] => Err(PlanError::NothingToRender),
        [only] => Ok(only),
        many => Err(PlanError::MultipleVideoTracks { count: many.len() }),
    }
}

fn audio_note(project: &Project) -> Vec<Note> {
    let tracks = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio && !track.clips.is_empty())
        .count();
    if tracks == 0 {
        Vec::new()
    } else {
        vec![Note::AudioNotMixed { tracks }]
    }
}

/// The asset a clip shows, if it is something this pipeline can decode today.
fn renderable_asset<'a>(project: &'a Project, clip: &Clip) -> Result<&'a Asset, PlanError> {
    let asset = project
        .asset(&clip.asset)
        .ok_or_else(|| PlanError::UnknownAsset {
            clip: clip.id.to_string(),
            asset: clip.asset.to_string(),
        })?;
    if asset.needs_generation() {
        return Err(PlanError::NotGenerated {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    if asset.kind == AssetKind::Text {
        return Err(PlanError::NeedsCompositor {
            clip: clip.id.to_string(),
            kind: asset.kind,
        });
    }
    if asset.path.is_none() {
        return Err(PlanError::NoMedia {
            clip: clip.id.to_string(),
            asset: asset.id.to_string(),
        });
    }
    Ok(asset)
}

/// Why a timeline cannot be sequenced into a render.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error("there is nothing to render: no video track has any clips")]
    NothingToRender,

    #[error(
        "this render covers one video track, but {count} have clips — \
         compositing tracks together arrives with the compositor"
    )]
    MultipleVideoTracks { count: usize },

    #[error("range {range} selects no frames of a timeline {timeline_end} long")]
    EmptyRange {
        range: FrameRange,
        timeline_end: Frames,
    },

    #[error("clip `{clip}` references asset `{asset}`, which is not in the assets table")]
    UnknownAsset { clip: String, asset: String },

    #[error(
        "clip `{clip}` shows asset `{asset}`, which has not been generated yet — \
         rendering sketches as slug cards is not implemented yet, so run `scorsese generate` first"
    )]
    NotGenerated { clip: String, asset: String },

    #[error("clip `{clip}` shows a {kind:?} asset, which needs the compositor to draw it")]
    NeedsCompositor { clip: String, kind: AssetKind },

    #[error("clip `{clip}` shows asset `{asset}`, which has no media file")]
    NoMedia { clip: String, asset: String },
}
