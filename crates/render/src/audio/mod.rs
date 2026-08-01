//! The audio half of a render: decode, mix, hand the result to the encoder.
//!
//! Structurally this mirrors what happens to picture. The plan cuts the
//! timeline where the *audible* set changes, one decoder runs per clip in a
//! stretch, and the samples are summed in our process — ffmpeg decodes and
//! encodes, and every decision about what is heard when is ours. That is Path B
//! with a different medium.
//!
//! A shot in one of these stretches can be a clip on a **video** track, when
//! its file has sound on it. Nothing here knows the difference: a source of
//! samples is a source of samples, and `-vn` on the decoder means a video file
//! arrives as exactly that.
//!
//! The mix is produced in full before the encoder starts, because ffmpeg takes
//! raw video on stdin and there is only one stdin. Writing it to a file next to
//! the output and handing that over as a second input keeps the render to two
//! processes at a time and needs no named pipes, which do not portably exist
//! everywhere this has to run.

pub mod gain;
pub mod level;
pub mod measure;
pub mod mix;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use scorsese_core::Frames;

use crate::error::{RenderError, Stage};
use crate::pipe::{AudioDecoder, AudioSource};
use crate::plan::{Plan, Segment, Shot};
use crate::report::{Note, StandIn};
use crate::settings::RenderSettings;
use crate::slug::Standing;
use crate::tools::Tools;

pub use gain::{Gain, path};
pub use level::{Levels, SoundLevels};
pub use measure::measure;
pub use mix::{CHANNELS, Mix, Ramp};
pub use scorsese_soundgen::level::{
    BandMeter, Bands, BandsDifference, Cut, Difference, Loudness, Meter, Profile, Profiler, Span,
};

/// A finished mix on disk, and the file's own undertaker.
///
/// Removed when dropped: it is scratch, it can be large, and a render that
/// fails part way through should not leave it behind. The output file is the
/// deliverable; this never is.
#[derive(Debug)]
pub struct Mixdown {
    path: PathBuf,
}

impl Mixdown {
    /// Where the samples are, for as long as this value is alive. Borrowed
    /// rather than handed out, because the file goes when it does.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Mixdown {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Mixes every audible clip in the plan into one file of raw samples.
///
/// `None` when the project has no audio at all, which is the difference between
/// a silent film and a film with a silent soundtrack: a render with nothing to
/// mix gets no audio stream, rather than an empty one.
pub fn mixdown(
    tools: &Tools,
    settings: &RenderSettings,
    plan: &Plan<'_>,
    project_root: &Path,
    out: &Path,
) -> Result<Option<(Mixdown, Vec<Note>, Levels)>, RenderError> {
    if plan.audio().is_empty() {
        return Ok(None);
    }

    let mixdown = Mixdown {
        path: scratch_path(out),
    };
    let file = File::create(&mixdown.path).map_err(|source| RenderError::Scratch {
        path: mixdown.path.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    let mut notes = Vec::new();
    let mut levels = Levels::new(settings.sample_rate.hz());

    for segment in plan.audio() {
        let mix = mix_segment(
            tools,
            settings,
            plan,
            project_root,
            segment,
            &mut notes,
            &mut levels,
        )?;
        // The mix is metered as it is written rather than afterwards: the file
        // is the only place the whole thing ever exists, and reading it back to
        // measure it would be a second pass over every sample of the render.
        levels.mix.feed(mix.samples());
        mix.write_to(&mut writer)
            .map_err(|source| RenderError::Pipe {
                stage: Stage::Mix,
                source,
            })?;
    }

    // Flushed and closed before this returns: the encoder is a separate process
    // that opens the file by name, and it can only read what has left our
    // buffers.
    writer.flush().map_err(|source| RenderError::Scratch {
        path: mixdown.path.clone(),
        source,
    })?;
    drop(writer);
    Ok(Some((mixdown, notes, levels)))
}

/// Sums one stretch of timeline, in which the audible set does not change.
fn mix_segment(
    tools: &Tools,
    settings: &RenderSettings,
    plan: &Plan<'_>,
    project_root: &Path,
    segment: &Segment<'_>,
    notes: &mut Vec<Note>,
    levels: &mut Levels,
) -> Result<Mix, RenderError> {
    let rate = settings.sample_rate.hz();
    let frames = plan.samples_of(segment, rate);
    let mut mix = Mix::silence(frames as usize);
    if frames == 0 {
        return Ok(mix);
    }

    // Frames of timeline per sample: the bridge between the grid keyframes are
    // written on and the grid samples arrive on.
    let per_sample = plan.timeline_fps().as_f64() / f64::from(rate);
    let mut decoded = Vec::new();

    for shot in &segment.layers {
        // A prompt with nothing generated contributes **silence**: the stretch
        // is already as long as it needs to be, so a narration sketch simply
        // adds nothing to it. Its card is on the picture for the same frames,
        // which is what makes a voice-over-driven cut watchable for free.
        // Registered even when there is nothing to decode, so a clip whose
        // narration was never generated is reported as silent rather than left
        // out. "This clip makes no sound" is the finding, and dropping the row
        // is how it goes unnoticed.
        let meter = levels.clip(shot.clip.id.as_str());
        let Some(file) = audio_file(shot, project_root, notes)? else {
            continue;
        };
        let source = audio_source(shot, file, plan, frames);
        let mut decoder = AudioDecoder::start(tools, &source, settings)?;
        let read = decoder.read(&mut decoded, frames as usize)?;
        decoder.finish()?;

        if read < frames as usize {
            notes.push(Note::AudioRanShort {
                clip: shot.clip.id.to_string(),
                missing_ms: (frames - read as u64) * 1_000 / u64::from(rate),
            });
        }
        // Metered as it is summed, and *after* its own volume keyframes: what
        // an author can act on is what this clip contributes to the mix, where
        // measuring the source instead would report the file rather than the
        // edit.
        mix.add(
            &decoded,
            Gain::of(&shot.clip.keyframes),
            Ramp {
                first: elapsed(segment.start, shot) as f64,
                per_sample,
            },
            meter,
        );
    }
    Ok(mix)
}

/// How far into its own clip this stretch of timeline begins.
fn elapsed(at: Frames, shot: &Shot<'_>) -> u64 {
    at.get().saturating_sub(shot.clip.start.get())
}

/// Where a shot's sound is on disk, or `None` when there is none to read.
///
/// The mix's half of the sketch lifecycle. A prompt that has not been generated
/// has nothing to decode and says so quietly; a file the project believes in
/// and cannot find says so in the report, because that one is a fault rather
/// than a stage. Either way the render carries on: silence is a thing a
/// soundtrack can contain, and refusing here would mean a preview cut cannot be
/// watched at all.
fn audio_file(
    shot: &Shot<'_>,
    project_root: &Path,
    notes: &mut Vec<Note>,
) -> Result<Option<PathBuf>, RenderError> {
    match crate::slug::standing(shot, project_root)? {
        Standing::Media(file) => Ok(Some(file)),
        Standing::Card(absent) => {
            if absent.is_a_problem() {
                notes.push(Note::GeneratedMissing {
                    clip: shot.clip.id.to_string(),
                    asset: shot.asset.id.to_string(),
                    stood_in: StandIn::Silence,
                });
            }
            Ok(None)
        }
    }
}

/// How much of a shot's sound to read, and from where in it.
fn audio_source(shot: &Shot<'_>, file: PathBuf, plan: &Plan<'_>, frames: u64) -> AudioSource {
    AudioSource {
        file,
        seek_seconds: plan.timeline_fps().seconds(shot.source_in),
        frames,
    }
}

/// Where to put the mix while it is being made: beside the output, hidden, and
/// named after it so two concurrent renders never collide.
fn scratch_path(out: &Path) -> PathBuf {
    let name = out
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "render".to_owned());
    out.with_file_name(format!(".{name}.scorsese-mix.pcm"))
}
