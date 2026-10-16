//! The passes one `generate` makes on this machine — shots, stills, lines, tracks —
//! for the CLI and the MCP tool alike (#1118).
//!
//! **Four passes, one ceiling, one total.** Shots, stills, narration and
//! music are separate passes because they are separate models and vendors, but the
//! budget is threaded from each into the next: a ceiling each pass checked on
//! its own would be worth three times what somebody set. That threading, which
//! pass runs, what a stopped one hands back and what landed to be measured
//! used to be written once per surface; they drifted (the CLI shortened no
//! clip for a while), so they live here and each surface only words the
//! result. The server queues each brief as a job instead and runs none of
//! this.
//!
//! **A key is asked for only by a pass that has work.** Whether a pass runs is
//! each provider's `pending`, which consults the **current brief's** output
//! file — never the asset's recorded `path`, which after an edit is the
//! previous generation's and still resolves. A project of nothing but
//! narration is never asked for a Veo key.
//!
//! **A pass that stops ends the run**, and what every pass did up to then is
//! handed back beside the reason: a ticket written before a failure is the
//! only record that money was spent (#1057). Saving the project is the
//! caller's, whatever happened.

use std::path::Path;
use std::time::Duration;

use scorsese_core::placing::{self, Shortened};
use scorsese_core::{AssetId, AssetKind, ProbeMedia, Project, Reprobe, probe_assets};

use crate::credentials::{Budget, CredentialError, Provider, resolve};
use crate::image::{self, GeminiProvider, ImageError, ImageProvider, Order};
use crate::music::{self, ElevenLabsComposer, MusicProvider};
use crate::speech::{self, ElevenLabsProvider, SpeechProvider};
use crate::video::{self, GenerationError, Run, VeoProvider, VideoProvider};

/// What a run was asked for, beyond the project.
#[derive(Debug, Clone, Copy)]
pub struct Asked {
    /// The ceiling, against what the project had spent before this run.
    pub budget: Budget,
    /// How long to wait for the shots before detaching.
    pub patience: Duration,
    /// Stills now, or as a half-price batch — which is then the only pass.
    pub order: Order,
}

/// Where each pass gets its provider, and how a wait waits.
///
/// A trait so the passes can be run against providers that cost nothing;
/// [`Keyed`] is the real one.
pub trait Vendors {
    /// Veo, keyed.
    fn video(&self) -> Result<Box<dyn VideoProvider>, CredentialError>;
    /// Gemini's images, keyed.
    fn image(&self) -> Result<Box<dyn ImageProvider>, CredentialError>;
    /// ElevenLabs, keyed.
    fn speech(&self) -> Result<Box<dyn SpeechProvider>, CredentialError>;
    /// ElevenLabs Music, keyed (#1264).
    fn music(&self) -> Result<Box<dyn MusicProvider>, CredentialError>;
    /// Sleeps between polls of the shots in flight.
    fn rest(&self, wait: Duration);
}

/// The real vendors, each keyed from this machine's credentials when its pass
/// first needs it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Keyed;

impl Vendors for Keyed {
    fn video(&self) -> Result<Box<dyn VideoProvider>, CredentialError> {
        Ok(Box::new(VeoProvider::new(
            &resolve(Provider::Gemini)?.secret,
        )))
    }

    fn image(&self) -> Result<Box<dyn ImageProvider>, CredentialError> {
        Ok(Box::new(GeminiProvider::new(
            &resolve(Provider::Gemini)?.secret,
        )))
    }

    fn speech(&self) -> Result<Box<dyn SpeechProvider>, CredentialError> {
        Ok(Box::new(ElevenLabsProvider::new(
            &resolve(Provider::ElevenLabs)?.secret,
        )))
    }

    fn music(&self) -> Result<Box<dyn MusicProvider>, CredentialError> {
        Ok(Box::new(ElevenLabsComposer::new(
            &resolve(Provider::ElevenLabs)?.secret,
        )))
    }

    fn rest(&self, wait: Duration) {
        std::thread::sleep(wait);
    }
}

/// What every pass did, a stopped one's included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    /// The shots, and what they committed.
    pub shots: Run,
    /// The stills.
    pub stills: Vec<(AssetId, image::Outcome)>,
    /// The narration.
    pub lines: Vec<(AssetId, speech::Outcome)>,
    /// The music (#1264).
    pub tracks: Vec<(AssetId, music::Outcome)>,
}

impl Default for Done {
    /// A run that has done nothing yet.
    fn default() -> Self {
        Self {
            shots: Run {
                outcomes: Vec::new(),
                spent_cents: 0,
            },
            stills: Vec::new(),
            lines: Vec::new(),
            tracks: Vec::new(),
        }
    }
}

impl Done {
    /// Whether no pass did anything at all.
    pub fn is_empty(&self) -> bool {
        self.shots.outcomes.is_empty()
            && self.stills.is_empty()
            && self.lines.is_empty()
            && self.tracks.is_empty()
    }

    /// What the stills committed.
    fn stills_spent(&self) -> u64 {
        self.stills.iter().map(|(_, o)| o.spent_cents()).sum()
    }

    /// What the whole run is estimated to have spent, in US cents — the one
    /// total that spans every provider. Our arithmetic, never a bill.
    pub fn spent_cents(&self) -> u64 {
        self.shots.spent_cents + self.stills_spent() + self.voiced_spent()
    }

    /// What the lines and the tracks committed — the ElevenLabs passes.
    fn voiced_spent(&self) -> u64 {
        self.lines.iter().map(|(_, o)| o.spent_cents()).sum::<u64>()
            + self
                .tracks
                .iter()
                .map(|(_, o)| o.spent_cents())
                .sum::<u64>()
    }

    /// The shots, lines and tracks that arrived on disk with a length nothing has
    /// measured yet — the ones whose clips may now outlast them.
    ///
    /// A cache hit is not one of them: its file was already there, and
    /// whatever ran when it first landed has had every chance to look at it.
    pub fn landed(&self) -> Vec<AssetId> {
        let shot = self
            .shots
            .outcomes
            .iter()
            .filter(|(_, outcome)| matches!(outcome, video::Outcome::Generated { .. }));
        let line = self
            .lines
            .iter()
            .filter(|(_, outcome)| matches!(outcome, speech::Outcome::Generated { .. }));
        let track = self
            .tracks
            .iter()
            .filter(|(_, outcome)| matches!(outcome, music::Outcome::Generated { .. }));
        shot.map(|(id, _)| id)
            .chain(line.map(|(id, _)| id))
            .chain(track.map(|(id, _)| id))
            .cloned()
            .collect()
    }

    /// Whether anything landed that has to be measured before the project
    /// is saved for good — a shot, a line or a still.
    pub fn unmeasured(&self) -> bool {
        !self.landed().is_empty() || self.stills.iter().any(|(_, o)| o.landed())
    }

    /// Probes everything unmeasured and shortens the clips of what
    /// [`landed`](Self::landed) that now outlast it, answering with each.
    ///
    /// **A generation has no measured shape until something looks**, and this
    /// crate has no decoder and must not grow one, so the prober is handed in.
    /// Measuring is also what can make the document refuse to load — a clip
    /// laid out over a sketch may outlast what came back — so the clips are
    /// fitted here, before the caller saves ([`placing::fit_to_sources`],
    /// #825, #1114).
    pub fn measure(
        &self,
        project: &mut Project,
        root: &Path,
        probe: &dyn ProbeMedia,
    ) -> Vec<Shortened> {
        probe_assets(project, root, probe, Reprobe::Skip);
        placing::fit_to_sources(project, &self.landed())
    }
}

/// The step a key was missing for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The shots pass.
    Shots,
    /// The stills pass, drawn now.
    Stills,
    /// The narration pass.
    Lines,
    /// The music pass.
    Tracks,
    /// The stills, ordered as a batch.
    Batch,
    /// Collecting what was in flight.
    Collect,
}

/// Why a run ended early. What it did before is in the [`Done`] beside it.
#[derive(Debug)]
pub enum Halt {
    /// A step with work to do found no key.
    Key {
        /// Which step.
        step: Step,
        /// Why there was none.
        error: CredentialError,
    },
    /// A pass stopped part-way: the whole story in words, so many sent and
    /// so many not, rather than the vendor's body (#1057, #1153).
    Stopped(String),
    /// Collecting the shots in flight failed.
    Shots(GenerationError),
    /// Collecting the stills waiting in a batch failed.
    Stills(ImageError),
}

/// Every pass that has work, each held to the ceiling with what the passes
/// before it committed counted as spent.
pub fn run(
    project: &mut Project,
    root: &Path,
    asked: &Asked,
    vendors: &dyn Vendors,
) -> (Done, Result<(), Halt>) {
    let mut done = Done::default();
    let outcome = passes(project, root, asked, vendors, &mut done);
    (done, outcome)
}

fn passes(
    project: &mut Project,
    root: &Path,
    asked: &Asked,
    vendors: &dyn Vendors,
    done: &mut Done,
) -> Result<(), Halt> {
    let key = |step| move |error| Halt::Key { step, error };
    // A batch is stills alone: its quote was refused had anything else been
    // waiting to be sent.
    if asked.order == Order::Batch {
        if image::pending(project, root) {
            let provider = vendors.image().map_err(key(Step::Batch))?;
            let ordered = image::batch(project, root, &*provider, asked.budget);
            kept(ordered, &mut done.stills)?;
        }
        return Ok(());
    }
    if video::pending(project, root) {
        let provider = vendors.video().map_err(key(Step::Shots))?;
        let rest = |wait| vendors.rest(wait);
        match video::generate_waiting(
            project,
            root,
            &*provider,
            asked.budget,
            asked.patience,
            rest,
        ) {
            Ok(run) => done.shots = run,
            Err(stopped) => {
                let says = stopped.says();
                done.shots = stopped.run;
                return Err(Halt::Stopped(says));
            }
        }
    }
    if image::pending(project, root) {
        let provider = vendors.image().map_err(key(Step::Stills))?;
        let budget = asked.budget.spend(done.shots.spent_cents);
        kept(
            image::generate(project, root, &*provider, budget),
            &mut done.stills,
        )?;
    }
    if speech::pending(project, root) {
        let provider = vendors.speech().map_err(key(Step::Lines))?;
        let budget = asked
            .budget
            .spend(done.shots.spent_cents + done.stills_spent());
        match speech::generate(project, root, &*provider, budget) {
            Ok(run) => done.lines = run,
            Err(stopped) => {
                let says = stopped.says();
                done.lines = stopped.run;
                return Err(Halt::Stopped(says));
            }
        }
    }
    if music::pending(project, root) {
        let provider = vendors.music().map_err(key(Step::Tracks))?;
        let budget = asked
            .budget
            .spend(done.shots.spent_cents + done.stills_spent() + done.voiced_spent());
        match music::generate(project, root, &*provider, budget) {
            Ok(run) => done.tracks = run,
            Err(stopped) => {
                let says = stopped.says();
                done.tracks = stopped.run;
                return Err(Halt::Stopped(says));
            }
        }
    }
    Ok(())
}

/// A stills run's outcomes, into `stills` — a stopped one's included — and
/// why it stopped, in words.
fn kept(
    run: Result<Vec<(AssetId, image::Outcome)>, image::Stopped>,
    stills: &mut Vec<(AssetId, image::Outcome)>,
) -> Result<(), Halt> {
    match run {
        Ok(run) => *stills = run,
        Err(stopped) => {
            let says = stopped.says();
            *stills = stopped.run;
            return Err(Halt::Stopped(says));
        }
    }
    Ok(())
}

/// Picks up whatever finished while nobody was watching: the shots in flight,
/// then the stills waiting in a batch. **Submits nothing**, so it cannot spend
/// — and asks for a key only when something is in flight to ask after.
pub fn collect(
    project: &mut Project,
    root: &Path,
    vendors: &dyn Vendors,
) -> (Done, Result<(), Halt>) {
    let mut done = Done::default();
    let outcome = collecting(project, root, vendors, &mut done);
    (done, outcome)
}

fn collecting(
    project: &mut Project,
    root: &Path,
    vendors: &dyn Vendors,
    done: &mut Done,
) -> Result<(), Halt> {
    let key = |error| Halt::Key {
        step: Step::Collect,
        error,
    };
    if in_flight(project, AssetKind::GeneratedVideo) {
        let provider = vendors.video().map_err(key)?;
        // A sweep spends nothing by construction, so its total is zero.
        done.shots.outcomes = video::collect(project, root, &*provider).map_err(Halt::Shots)?;
    }
    if in_flight(project, AssetKind::GeneratedImage) {
        let provider = vendors.image().map_err(key)?;
        done.stills = image::collect(project, root, &*provider).map_err(Halt::Stills)?;
    }
    Ok(())
}

/// Whether an asset of `kind` carries a ticket. Narration never does.
fn in_flight(project: &Project, kind: AssetKind) -> bool {
    project
        .assets
        .iter()
        .any(|asset| asset.kind == kind && asset.operation.is_some())
}
