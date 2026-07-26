//! Turning a timeline into an ordered list of things to decode.
//!
//! This is the part of rendering that involves no ffmpeg and no bytes: read
//! the project, work out what occupies every frame of the requested range, and
//! say how many output frames each stretch is worth. Keeping it separate is
//! what lets the sequencing be tested exhaustively without encoding anything.

mod range;
mod segments;

use scorsese_core::{Asset, AssetKind, Clip, Fps, Frames, Project, Track, TrackKind};

use crate::report::Note;

pub use range::{FrameRange, FrameRangeError};

/// A clip resolved to the asset it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot<'a> {
    pub clip: &'a Clip,
    pub asset: &'a Asset,
    /// Where in the source to start, in frames of the **timeline** grid — the
    /// clip's own `source_in`, plus however far into the clip this stretch of
    /// timeline begins. A clip cut across several segments by a boundary on
    /// another track resumes from the right frame in each.
    pub source_in: Frames,
}

/// One stretch of the timeline over which the visible set does not change.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment<'a> {
    /// Where this stretch begins, in timeline frames.
    pub start: Frames,
    /// How many timeline frames it covers.
    pub duration: Frames,
    /// What is visible throughout, **bottom track first**.
    ///
    /// Empty means a gap: nothing on any video track. Gaps render black rather
    /// than being skipped — a hole in the middle of an edit is two seconds of
    /// black, not two seconds of missing time.
    pub layers: Vec<Shot<'a>>,
}

impl Segment<'_> {
    /// The frame just past this stretch's last one.
    pub fn end(&self) -> Frames {
        self.start + self.duration
    }

    /// True when nothing at all is on screen here.
    pub fn is_gap(&self) -> bool {
        self.layers.is_empty()
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
    audio: Vec<Segment<'a>>,
    notes: Vec<Note>,
}

impl<'a> Plan<'a> {
    /// Sequences a project's tracks over `range`, for an output at `out_fps`.
    ///
    /// **Picture decides how long the render is.** A music bed running past the
    /// last shot is trimmed rather than extending the file, because the thing
    /// being produced is a video: an edit ends when the last thing you can see
    /// ends. Audio lost that way is reported, never dropped in silence.
    ///
    /// What is *heard* is not simply the audio tracks. A clip on a video track
    /// whose file carries an audio stream is mixed alongside them, at its own
    /// keyframed `volume` like anything else — the sound on a shot is part of
    /// the shot. Whether a file has that stream is read from the assets table,
    /// so this stays a function of the project with no ffprobe in it.
    pub fn build(project: &'a Project, out_fps: Fps, range: FrameRange) -> Result<Self, PlanError> {
        let tracks = segments::tracks_of(project, TrackKind::Video);
        if tracks.is_empty() {
            return Err(PlanError::NothingToRender);
        }
        let timeline_end = segments::timeline_end(&tracks);
        let audible = |track: &Track, clip: &Clip| segments::is_audible(project, track, clip);
        let audio_tracks =
            segments::taking_part(project, &[TrackKind::Video, TrackKind::Audio], audible);

        let mut notes = Vec::new();
        // Only the audio *tracks* can outlast the picture: sound that came off
        // a video clip ends when that clip's picture does, by construction.
        let audio_end = segments::timeline_end(&segments::tracks_of(project, TrackKind::Audio));
        if audio_end > timeline_end {
            notes.push(Note::AudioTrimmed {
                audio_end,
                timeline_end,
            });
        }
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

        Ok(Self {
            start,
            end,
            timeline_fps: project.timeline_fps,
            out_fps,
            segments: segments::build(project, &tracks, start, end, segments::anything)?,
            audio: segments::build(project, &audio_tracks, start, end, audible)?,
            notes,
        })
    }

    pub fn segments(&self) -> &[Segment<'a>] {
        &self.segments
    }

    /// The audible stretches, cut where the audible set changes — the same
    /// shape as [`Plan::segments`], because a stack of sounds playing at once
    /// is the same problem as a stack of pictures. A shot here can be a clip on
    /// a video track: its picture is in [`Plan::segments`] and its sound is
    /// here, and neither knows about the other.
    pub fn audio(&self) -> &[Segment<'a>] {
        &self.audio
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub const fn out_fps(&self) -> Fps {
        self.out_fps
    }

    /// The grid the project was authored against.
    pub const fn timeline_fps(&self) -> Fps {
        self.timeline_fps
    }

    /// How many frames the render writes in total.
    pub fn out_frames(&self) -> u64 {
        self.out_index(self.end)
    }

    /// The most layers any one stretch stacks, which is how many frame buffers
    /// a render needs.
    pub fn widest_stack(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.layers.len())
            .max()
            .unwrap_or(0)
    }

    /// How many output frames one segment is worth.
    ///
    /// Derived from the segment's timeline **boundaries** rather than from its
    /// duration, so segment counts always sum to the render's total. Conforming
    /// each duration on its own would round each one independently and drift.
    pub fn out_frames_of(&self, segment: &Segment<'_>) -> u64 {
        self.out_index(segment.end()) - self.out_index(segment.start)
    }

    /// Which timeline frame the `index`th output frame of a segment shows.
    ///
    /// The inverse of the conform that decided the segment's frame count, and
    /// the reason it is needed: keyframes are timed against the timeline grid,
    /// so the compositor has to be told which instant it is drawing — not which
    /// output frame it happens to be on.
    pub fn timeline_frame_of(&self, segment: &Segment<'_>, index: u64) -> Frames {
        segment.start + self.timeline_fps.conform(Frames(index), self.out_fps)
    }

    /// How many sample-frames of audio one segment is worth, at `rate`.
    ///
    /// Derived from boundaries for the same reason [`Plan::out_frames_of`] is:
    /// rounding each segment's duration on its own would drift, and audio that
    /// drifts against picture is the one error nobody forgives.
    pub fn samples_of(&self, segment: &Segment<'_>, rate: u32) -> u64 {
        self.sample_index(segment.end(), rate) - self.sample_index(segment.start, rate)
    }

    /// How many sample-frames the whole mix runs to, at `rate`.
    pub fn total_samples(&self, rate: u32) -> u64 {
        self.sample_index(self.end, rate)
    }

    /// The output frame a timeline frame lands on, counted from the start of
    /// the render.
    fn out_index(&self, at: Frames) -> u64 {
        let offset = Frames(at.get().saturating_sub(self.start.get()));
        self.out_fps.conform(offset, self.timeline_fps).get()
    }

    /// The sample a timeline frame lands on, counted from the start of the
    /// render. Exact until the final rounding, like every other conform here.
    fn sample_index(&self, at: Frames, rate: u32) -> u64 {
        let offset = u128::from(at.get().saturating_sub(self.start.get()));
        let numerator = offset * u128::from(self.timeline_fps.den()) * u128::from(rate);
        let denominator = u128::from(self.timeline_fps.num());
        u64::try_from((numerator + denominator / 2) / denominator).unwrap_or(u64::MAX)
    }
}

/// Why a timeline cannot be sequenced into a render.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error("there is nothing to render: no video track has any clips")]
    NothingToRender,

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
