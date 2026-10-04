//! One render, running on its own thread while the window carries on.
//!
//! The pattern is [`Probing`](crate::project::probing::Probing)'s and the
//! generate worker's: a copy of the document goes to a thread, an answer comes
//! back over a channel, and the repaint loop asks after it without ever
//! blocking. What this adds is the two handles the renderer publishes —
//! [`Progress`] to read how far it has got, [`Cancel`] to stop it — both of
//! which are cloned between this side and the render and touched from either
//! without a lock.
//!
//! **The render works from a copy taken when it starts.** That is what lets the
//! window stay editable while it runs: a change made mid-render is saved to the
//! document as usual and cannot reach a render that already holds its own
//! project. The file is the edit as it stood when Render was pressed.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use scorsese_core::Project;
use scorsese_render::{
    Cancel, FrameRange, Progress, Reading, RenderError, RenderSettings, Renderer, Tools, say,
};

/// How a render came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ended {
    /// The file is complete; the sentence says what is in it.
    Wrote(String),
    /// Stop was pressed. The renderer removes a half-written file (#647), so
    /// there is nothing at the path.
    Stopped,
    /// It failed, and this is why — worded by the renderer, for a person.
    Failed(String),
}

/// Where the readout comes from: a render running, or a value held still.
enum Gauge {
    /// The handle the render publishes to.
    Live(Progress),
    /// A fixed reading, for a snapshot of the popup part way through — a
    /// real render would be wherever the machine happened to have got it.
    Held(Reading),
}

/// A render in flight, or finished and not yet dismissed.
pub(crate) struct Job {
    gauge: Gauge,
    cancel: Cancel,
    out: PathBuf,
    answer: Option<Receiver<Ended>>,
    ended: Option<Ended>,
}

impl Job {
    /// Starts rendering a copy of `project` to `out`, the whole timeline.
    pub(crate) fn start(
        project: &Project,
        root: &Path,
        settings: RenderSettings,
        out: PathBuf,
    ) -> Self {
        let progress = Progress::new();
        let cancel = Cancel::new();
        let (sender, answer) = channel();
        let copy = project.clone();
        let root = root.to_path_buf();
        let (watched, stop, to) = (progress.clone(), cancel.clone(), out.clone());
        std::thread::spawn(move || {
            let _ = sender.send(run(&copy, &root, settings, &to, watched, stop));
        });
        Self {
            gauge: Gauge::Live(progress),
            cancel,
            out,
            answer: Some(answer),
            ended: None,
        }
    }

    /// A job that never runs and always reads `reading` — the popup as it
    /// looks part way through, for a test to draw.
    pub(crate) fn held(reading: Reading, out: PathBuf) -> Self {
        Self {
            gauge: Gauge::Held(reading),
            cancel: Cancel::new(),
            out,
            answer: None,
            ended: None,
        }
    }

    /// Where the render is now.
    pub(crate) fn reading(&self) -> Reading {
        match &self.gauge {
            Gauge::Live(progress) => progress.read(),
            Gauge::Held(reading) => *reading,
        }
    }

    /// Where the file is going.
    pub(crate) fn out(&self) -> &Path {
        &self.out
    }

    /// Whether the render has yet to answer, as of the last look.
    pub(crate) fn running(&self) -> bool {
        self.ended.is_none()
    }

    /// Asks the render to stop. It lands within a frame once drawing.
    pub(crate) fn stop(&self) {
        self.cancel.cancel();
    }

    /// Whether Stop has been pressed and the render has not answered yet.
    pub(crate) fn stopping(&self) -> bool {
        self.ended.is_none() && self.cancel.is_cancelled()
    }

    /// How it came out, once it has. Never blocks.
    pub(crate) fn ended(&mut self) -> Option<&Ended> {
        if self.ended.is_none()
            && let Some(answer) = &self.answer
        {
            match answer.try_recv() {
                Ok(ended) => self.ended = Some(ended),
                Err(TryRecvError::Empty) => {}
                // The thread died without answering, which only a panic does.
                Err(TryRecvError::Disconnected) => {
                    self.ended = Some(Ended::Failed(String::from(
                        "the render stopped unexpectedly",
                    )));
                }
            }
        }
        self.ended.as_ref()
    }
}

/// The render itself, on the worker thread.
fn run(
    project: &Project,
    root: &Path,
    settings: RenderSettings,
    out: &Path,
    progress: Progress,
    cancel: Cancel,
) -> Ended {
    let tools = match Tools::discover() {
        Ok(tools) => tools,
        Err(error) => return Ended::Failed(error.to_string()),
    };
    let outcome = Renderer::new(&tools, settings)
        .with_progress(progress)
        .with_cancel(cancel)
        .render(project, root, FrameRange::ALL, out);
    match outcome {
        Ok(report) => Ended::Wrote(say::written(&report)),
        Err(RenderError::Cancelled { .. }) => Ended::Stopped,
        Err(error) => Ended::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
