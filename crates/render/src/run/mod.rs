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

#[cfg(test)]
mod cancelled;
mod pages;
#[cfg(test)]
mod progressed;
mod segment;
mod still;

use std::path::Path;

use scorsese_compositor::Frame;
use scorsese_core::{Frames, Project};

use crate::audio;
use crate::cancel::Cancel;
use crate::error::RenderError;
use crate::held::Loops;
use crate::page::{self, Chrome};
use crate::pipe::{Encoder, encode_mix};
use crate::plan::{FrameRange, Plan};
use crate::preview::Preview;
use crate::progress::{Phase, Progress};
use crate::raster::Sizes;
use crate::report::{Note, RenderReport};
use crate::settings::RenderSettings;
use crate::tools::Tools;
use crate::workers::Workers;

use pages::{Browser, Pages};
use segment::{Pass, Stage};

/// Renders projects with one set of settings.
pub struct Renderer<'a> {
    tools: &'a Tools,
    settings: RenderSettings,
    workers: Workers,
    preview: Option<Preview>,
    cancel: Cancel,
    progress: Progress,
    chrome: Option<Chrome>,
    capturing: bool,
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
            preview: None,
            cancel: Cancel::new(),
            progress: Progress::new(),
            chrome: None,
            capturing: true,
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

    /// Draws a **preview** rather than a delivery: `native` clips scaled with
    /// the raster the quality chose, and the preview's proxies decoded in place
    /// of their originals when the quality reads them ([`crate::preview`]).
    ///
    /// The one door a proxy comes in by. Nothing that delivers a file — `scorsese
    /// render`, the MCP `render` tool, the web app's finished renders — calls
    /// it, so a delivered file always reads originals.
    pub fn with_preview(self, preview: Preview) -> Self {
        Self {
            preview: Some(preview),
            ..self
        }
    }

    /// Stops when `cancel` is tripped, from whichever thread trips it.
    ///
    /// Looked at before each frame is encoded and between the stages before
    /// that — the mix, the loudness rehearsal — so a stop lands within a frame
    /// of being asked for once the picture is under way. What a stopped render
    /// returns is [`RenderError::Cancelled`], saying how far it got, and it
    /// leaves no file at `out`: a half-written one is removed (#647).
    pub fn with_cancel(self, cancel: Cancel) -> Self {
        Self { cancel, ..self }
    }

    /// Publishes how far each render has got to `progress`, for whoever holds
    /// a clone of it to read from another thread ([`Progress`] has what the
    /// percentage means).
    ///
    /// A render starts it over at [`Phase::Preparing`], counts frames into it
    /// as they are encoded, and leaves it at [`Phase::Done`] once the report is
    /// in hand. A render that fails or is cancelled leaves it where it stopped.
    pub fn with_progress(self, progress: Progress) -> Self {
        Self { progress, ..self }
    }

    /// Captures `html` clips with this browser rather than whichever
    /// [`Chrome::discover`] finds — for a caller that ships its own, or has
    /// already checked one. Without it, the browser is looked for only when a
    /// render has a page on screen.
    pub fn with_chrome(self, chrome: Chrome) -> Self {
        Self {
            chrome: Some(chrome),
            ..self
        }
    }

    /// Draws `html` clips only from captures already in `cache/`, and as their
    /// slug cards otherwise — never starting a browser, and so never making the
    /// caller wait while a page is captured. For a window, which captures in
    /// the background ([`Renderer::page_requests`], then [`crate::page::capture`])
    /// and draws again once a capture lands. Without a browser given
    /// ([`Renderer::with_chrome`]) every page is a card, since which build would
    /// capture it is part of where its capture is kept.
    pub fn without_capturing(self) -> Self {
        Self {
            capturing: false,
            ..self
        }
    }

    fn browser(&self) -> Browser<'_> {
        Browser {
            chrome: self.chrome.as_ref(),
            capturing: self.capturing,
        }
    }

    /// Every page capture rendering `project` at these settings would need,
    /// once each — what to hand [`crate::page::capture`] ahead of time.
    pub fn page_requests(&self, project: &Project) -> Result<Vec<page::Request>, RenderError> {
        let plan = Plan::build(project, self.settings.fps, FrameRange::ALL)?;
        Ok(pages::requests(&self.settings, &plan))
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
        self.progress.start();
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
        let of: u64 = plan
            .segments()
            .iter()
            .map(|segment| plan.out_frames_of(segment))
            .sum();
        self.progress.planned(of);
        // Between stages too, not only between frames: on a long timeline the
        // mix and its rehearsal are minutes of their own before a frame is
        // drawn, and a stop asked for then should not wait them out.
        let stopped = || Err(RenderError::Cancelled { written: 0, of });
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
        // Pages are captured before anything is mixed or drawn: a capture is
        // the slowest thing a render can do, and its frames are what the
        // drawing will decode. A page that cannot be captured is a card and a
        // note, never a refusal.
        let pages = if picture {
            let (pages, page_notes) = Pages::capture(
                self.tools,
                self.browser(),
                &self.settings,
                &plan,
                project_root,
            );
            notes.extend(page_notes);
            pages
        } else {
            Pages::default()
        };
        let sizes = if picture {
            Some((
                Sizes::measure(self.tools, &plan, project_root)?,
                Loops::measure(self.tools, &plan, project_root),
            ))
        } else {
            None
        };

        if self.cancel.is_cancelled() {
            return stopped();
        }
        self.progress.enter(Phase::Mixing);
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

        if self.cancel.is_cancelled() {
            return stopped();
        }
        self.progress.enter(if picture {
            Phase::Drawing
        } else {
            Phase::Finishing
        });
        let written = match (&sizes, mix) {
            (Some((sizes, loops)), _) => {
                let (written, picture_notes) =
                    self.picture(&plan, (sizes, loops, &pages), project_root, mix, (out, of))?;
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
        let report = RenderReport {
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
        };
        self.progress.enter(Phase::Done);
        Ok(report)
    }

    /// Composites every frame of `plan` and encodes it to `out`, with the
    /// finished `mix` muxed in when there is one. Hands back how many frames
    /// were written and what drawing them noticed.
    ///
    /// `of` is how many frames that will be, said in a cancel. A render that
    /// stops here for any reason — a cancel, a decoder that failed — takes the
    /// encoder down with it and removes the file it had begun.
    fn picture(
        &self,
        plan: &Plan<'_>,
        (sizes, loops, pages): (&Sizes, &Loops, &Pages),
        project_root: &Path,
        mix: Option<&Path>,
        (out, of): (&Path, u64),
    ) -> Result<(u64, Vec<Note>), RenderError> {
        let mut encoder = Encoder::start(self.tools, &self.settings, mix, out)?;
        let mut stage = Stage::new();
        let pass = Pass {
            tools: self.tools,
            settings: self.settings,
            plan,
            sizes,
            loops,
            pages,
            project_root,
            workers: self.workers,
            preview: self.preview.as_ref(),
        };
        let mut written = 0;
        let mut notes = Vec::new();
        let mut drawn = || -> Result<(), RenderError> {
            for segment in plan.segments() {
                notes.extend(pass.render(
                    segment,
                    plan.out_frames_of(segment),
                    &mut stage,
                    &mut |frame| {
                        // Before the frame rather than after: a cancel asked
                        // for while it was being drawn should not cost the
                        // encode of it too.
                        if self.cancel.is_cancelled() {
                            return Err(RenderError::Cancelled { written, of });
                        }
                        encoder.write(frame)?;
                        written += 1;
                        self.progress.drew(written);
                        Ok(())
                    },
                )?);
            }
            Ok(())
        };
        if let Err(stopped) = drawn() {
            encoder.abandon();
            return Err(stopped);
        }
        self.progress.enter(Phase::Finishing);
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
        self.still_noted(project, project_root, at)
            .map(|(frame, _)| frame)
    }

    /// [`Renderer::still`], and what drawing it noticed that the picture
    /// cannot say: a page that could not be captured, or one that asked for
    /// something it was not given (#777). The notes a render's report would
    /// carry for the pages on screen at `at`, for a caller that tells somebody
    /// — an agent cannot fix a page it is not told about. A window redrawing
    /// a scrub has nobody to tell, and reads only whether a page was not
    /// captured yet, to capture the one under its playhead first (#875).
    pub fn still_noted(
        &self,
        project: &Project,
        project_root: &Path,
        at: Frames,
    ) -> Result<(Frame, Vec<Note>), RenderError> {
        still::compose(
            self.tools,
            self.browser(),
            self.settings,
            self.preview.as_ref(),
            project,
            project_root,
            at,
        )
    }
}
