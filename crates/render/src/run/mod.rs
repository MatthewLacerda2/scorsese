//! Driving a plan through the pipes: decode → composite → encode.
//!
//! This module is a render's outer shape — probe, plan, mix, encode, and the
//! walk over the plan's segments. What happens *inside* one stretch of
//! timeline, where the decoders run and each frame is drawn, is `segment`.
//!
//! `still` is the same machinery with the encoder taken out: one frame, handed
//! back as pixels, for whoever is looking at the edit rather than delivering
//! it. It shares `segment` rather than paralleling it, so a preview cannot draw
//! the picture differently from the file.

mod segment;
mod still;

use std::path::Path;

use scorsese_compositor::Frame;
use scorsese_core::{Frames, Project};

use crate::audio;
use crate::error::RenderError;
use crate::pipe::{Encoder, encode_mix};
use crate::plan::{FrameRange, Plan};
use crate::raster::Sizes;
use crate::report::{Note, RenderReport};
use crate::settings::RenderSettings;
use crate::tools::Tools;
use crate::workers::Workers;

use segment::{Pass, Stage};

/// Renders projects with one set of settings.
pub struct Renderer<'a> {
    tools: &'a Tools,
    settings: RenderSettings,
    workers: Workers,
}

impl<'a> Renderer<'a> {
    /// Borrows the tools rather than discovering its own, so the cost of
    /// checking ffmpeg is paid once however many renders follow.
    ///
    /// Composites on [`Workers::available`] threads unless
    /// [`Renderer::with_workers`] says otherwise.
    pub fn new(tools: &'a Tools, settings: RenderSettings) -> Self {
        Self {
            tools,
            settings,
            workers: Workers::default(),
        }
    }

    /// Composites on `workers` threads rather than as many as the machine has
    /// to spare.
    ///
    /// A separate knob from [`RenderSettings`] on purpose: everything in there
    /// decides something about the delivered file, and this decides nothing
    /// about it. The same project rendered on one thread and on eight produces
    /// the same frames — that is the property the golden renders hold this to.
    pub fn with_workers(self, workers: Workers) -> Self {
        Self { workers, ..self }
    }

    /// Renders `range` of `project` to `out`.
    ///
    /// Expects a project that already validated — [`Project::load`] does that,
    /// and rendering an incoherent timeline is not a thing worth defining.
    ///
    /// A format with no picture in it ([`crate::OutputFormat::has_picture`])
    /// never reaches the compositor: the mix is made exactly as it would be
    /// for the video, and encoded on its own.
    pub fn render(
        &self,
        project: &Project,
        project_root: &Path,
        range: FrameRange,
        out: &Path,
    ) -> Result<RenderReport, RenderError> {
        let picture = self.settings.format.has_picture();
        // First, before anything is probed or mixed: an encoder this ffmpeg
        // was built without is a refusal that costs nothing now and an encode
        // later.
        let codec = self.settings.format.audio();
        if let Some(library) = codec.library()
            && !self.tools.has_encoder(library)?
        {
            return Err(RenderError::MissingEncoder {
                codec: codec.name(),
                library,
            });
        }
        // What a video clip's file has on it decides whether its sound is
        // mixed, so anything the project never recorded is found out here —
        // before the plan, which is a pure function of the document.
        let (project, probe_notes) = crate::probe::fill_media(self.tools, project, project_root);
        let plan = if picture {
            Plan::build(&project, self.settings.fps, range)?
        } else {
            Plan::build_sound(&project, self.settings.fps, range)?
        };
        let mut notes = plan.notes().to_vec();
        notes.extend(probe_notes);
        // Said at the start, about the whole project rather than the range: a
        // keyframe track nothing resolves does not stop this render, it just
        // never does anything, and that is the sort of thing a render should
        // mention before it spends minutes producing exactly the wrong fade.
        notes.extend(
            crate::properties::unknown_in(&project)
                .into_iter()
                .map(Note::from),
        );
        // Before anything is spawned: a clip asking for its source's own size
        // needs that size established, and this is the cheap place to fail if
        // it cannot be. What the probe above filled in is answer enough for
        // most of them, so this rarely spawns anything of its own. A delivery
        // with no picture draws nothing, so it has no sizes to establish.
        let sizes = if picture {
            Some(Sizes::measure(self.tools, &plan, project_root)?)
        } else {
            None
        };

        // Sound before picture, because the encoder needs the finished mix as
        // an input file. It is also the cheaper half: a mix that fails on a
        // missing music file should fail before we spend minutes encoding.
        let mixed = audio::mixdown(self.tools, &self.settings, &plan, project_root, out)?;
        let mix = mixed.as_ref().map(|(mixdown, _, _)| mixdown.path());
        let has_audio = mix.is_some();
        if let Some((_, mix_notes, _)) = &mixed {
            notes.extend(mix_notes.iter().cloned());
        }
        // Taken before `mixed` is dropped, which is what removes the scratch
        // file. The numbers outlive the samples they were measured from.
        let levels = mixed.as_ref().map(|(_, _, levels)| levels.finish());

        // Rehearsed through the delivery's codec before the picture is spent
        // on: a lossy codec overshoots by an amount only the material decides,
        // and this is the last moment the mix can still be turned down to
        // leave it room. A no-op for a lossless codec or a mix that fits.
        let trim = match mix {
            Some(mix) => audio::headroom::fit(self.tools, &self.settings, mix, out)?,
            None => None,
        };

        let written = match (&sizes, mix) {
            (Some(sizes), _) => {
                let (written, picture_notes) =
                    self.picture(&plan, sizes, project_root, mix, out)?;
                notes.extend(picture_notes);
                written
            }
            // The whole of a sound-only render's encode: the mix, in the
            // delivery's codec — the same call that rehearsed it above, so
            // what was measured there is what is written here.
            (None, Some(mix)) => {
                encode_mix(self.tools, &self.settings, mix, out)?;
                0
            }
            (None, None) => return Err(RenderError::NothingAudible),
        };
        // Only now is the scratch mix expendable: dropping it removes the file,
        // and the encoder has been reading from it until this point.
        drop(mixed);
        // Read back out of the file as delivered, because the mix's own level
        // is a statement about the samples we handed the encoder, and a lossy
        // one hands back different ones.
        let delivered = if has_audio {
            Some(audio::headroom::measure(self.tools, out)?)
        } else {
            None
        };
        Ok(RenderReport {
            frames: written,
            fps: self.settings.fps,
            resolution: picture.then_some(self.settings.resolution),
            seconds_of_audio: has_audio.then(|| {
                plan.total_samples(self.settings.sample_rate.hz()) as f64
                    / f64::from(self.settings.sample_rate.hz())
            }),
            levels,
            delivered,
            trim,
            notes,
            description: crate::describe::Description::of(&plan),
        })
    }

    /// Composites every frame of `plan` and encodes it to `out`, with the
    /// finished `mix` muxed in when there is one. Hands back how many frames
    /// were written and what drawing them noticed.
    fn picture(
        &self,
        plan: &Plan<'_>,
        sizes: &Sizes,
        project_root: &Path,
        mix: Option<&Path>,
        out: &Path,
    ) -> Result<(u64, Vec<Note>), RenderError> {
        let mut encoder = Encoder::start(self.tools, &self.settings, mix, out)?;
        let mut stage = Stage::new();
        let pass = Pass {
            tools: self.tools,
            settings: self.settings,
            plan,
            sizes,
            project_root,
            workers: self.workers,
        };
        let mut written = 0;
        let mut notes = Vec::new();
        for segment in plan.segments() {
            let frames = plan.out_frames_of(segment);
            notes.extend(pass.render(segment, frames, &mut stage, &mut |frame| {
                encoder.write(frame)
            })?);
            written += frames;
        }
        encoder.finish()?;
        Ok((written, notes))
    }

    /// One frame of `project` at timeline frame `at`, composited and handed
    /// back — no encoder, no file, no sound.
    ///
    /// What a window shows when someone scrubs, and the reason it can be
    /// trusted: this is [`Renderer::render`]'s own plan, decoders and
    /// compositor, stopped one step before the encode. A slug card here is the
    /// slug card the delivered file would have, at the raster the settings ask
    /// for — which is how a preview cut of prompts nobody has paid for can be
    /// watched at all.
    ///
    /// It is not cheap. Every call spawns an ffmpeg per layer with a source,
    /// so a caller redrawing a window is expected to remember the frame it got
    /// and ask again only when the instant it wants changes.
    pub fn still(
        &self,
        project: &Project,
        project_root: &Path,
        at: Frames,
    ) -> Result<Frame, RenderError> {
        still::compose(self.tools, self.settings, project, project_root, at)
    }
}
