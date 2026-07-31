//! Turning a timeline into an ordered list of things to decode.
//!
//! This is the part of rendering that involves no ffmpeg and no bytes: read
//! the project, work out what occupies every frame of the requested range, and
//! say how many output frames each stretch is worth. Keeping it separate is
//! what lets the sequencing be tested exhaustively without encoding anything.

mod error;
mod range;
mod segments;

use scorsese_core::{Asset, Clip, Fps, Frames, Project, Track, TrackId, TrackKind};

use crate::report::Note;

pub use error::PlanError;
pub use range::{FrameRange, FrameRangeError};

/// What a clip puts on screen, or into the mix.
///
/// The whole of the sketch lifecycle as the renderer sees it, and it is decided
/// from the **document**: an asset either has media the project believes in, or
/// it does not and a card stands in. Whether the file that belief names is
/// actually on disk is a question for later, in [`crate::slug`], where opening
/// files is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Showing {
    /// The asset's own media, decoded from the file it names — or, for a text
    /// asset, the content it carries inline.
    Media,
    /// A slug card: the prompt on a gray card, because there is nothing
    /// generated to show. What `sketch`, `queued` and `stale` all render as,
    /// and what makes a full preview cut cost nothing.
    Card,
}

/// A clip resolved to the asset it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot<'a> {
    /// The track the clip sits on. Carried because *which lane* a shot is in
    /// is half of what a stack means — the layer order for picture, and the
    /// only way a reader can tell the narration from the music.
    pub track: &'a TrackId,
    /// The clip as authored — its placement, fit, and keyframes.
    pub clip: &'a Clip,
    /// The entry the clip's asset id resolved to. Looked up once here so
    /// nothing downstream has to carry the assets table around.
    pub asset: &'a Asset,
    /// Media or a slug card. Resolved here, once, so that no later stage has
    /// to re-derive the lifecycle rules from `state` and `path`.
    pub showing: Showing,
    /// Where in the source to start, in frames of the **timeline** grid — the
    /// clip's own `source_in`, plus however far into the clip this stretch of
    /// timeline begins, taken at the clip's speed. A clip cut across several
    /// segments by a boundary on another track resumes from the right frame in
    /// each.
    ///
    /// Fractional, and that is what a clip's speed costs: at 1.5× a segment
    /// beginning 5 timeline frames into a clip opens 7.5 frames into its
    /// source, which is a real point in the media and not a frame of the
    /// timeline. Rounding it here would leave a resumed segment up to half a
    /// frame from where the one before it stopped — a jump in the picture and a
    /// click in the mix, at every cut on another track.
    pub source_in: f64,
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
    ///
    /// And what is *seen* is not simply the video tracks. A narration prompt
    /// nobody has generated yet has a slug card, and a card is picture: it goes
    /// on the output above the shots it narrates, for exactly as long as its
    /// clip lasts, so that a cut driven by its voice-over can be watched before
    /// a word of it has been paid for.
    pub fn build(project: &'a Project, out_fps: Fps, range: FrameRange) -> Result<Self, PlanError> {
        let tracks = segments::tracks_of(project, TrackKind::Video);
        if tracks.is_empty() {
            return Err(PlanError::NothingToRender);
        }
        let timeline_end = segments::timeline_end(&tracks);
        let audible = |track: &Track, clip: &Clip| segments::is_audible(project, track, clip);
        let audio_tracks =
            segments::taking_part(project, &[TrackKind::Video, TrackKind::Audio], audible);
        let visible = |track: &Track, clip: &Clip| segments::is_visible(project, track, clip);
        let mut picture_tracks = tracks.clone();
        // After the video tracks rather than in document order: a narration
        // card belongs over the picture, and nothing on an audio track can be
        // what a shot is composited *onto*.
        picture_tracks.extend(segments::taking_part(project, &[TrackKind::Audio], visible));

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
            segments: segments::build(project, &picture_tracks, start, end, visible)?,
            audio: segments::build(project, &audio_tracks, start, end, audible)?,
            notes,
        })
    }

    /// The visible stretches, in order and covering the render's range with no
    /// holes — a gap is a segment with no layers, not an absence.
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

    /// What sequencing noticed on the way — trimmed audio, a clamped range.
    /// Sequencing never refuses a project over these, so the caller carries
    /// them into its [`crate::report::RenderReport`].
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// The grid the render writes on, which the timeline is conformed to.
    pub const fn out_fps(&self) -> Fps {
        self.out_fps
    }

    /// The first timeline frame the render covers — frame 0 of the file that
    /// comes out, which is not frame 0 of the timeline when a range was asked
    /// for.
    pub const fn start(&self) -> Frames {
        self.start
    }

    /// The frame just past the last one the render covers.
    pub const fn end(&self) -> Frames {
        self.end
    }

    /// Which frame of the delivered file a timeline instant lands on.
    ///
    /// The map anyone reading the output needs: the description talks about the
    /// timeline, the file is counted from the start of the render, and pulling
    /// a still out of it means converting between the two exactly once.
    pub fn out_frame_of(&self, at: Frames) -> u64 {
        self.out_index(at)
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
