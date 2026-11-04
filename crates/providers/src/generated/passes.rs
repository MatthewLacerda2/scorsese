//! The passes one `generate` makes on this machine — shots, stills, lines, tracks, sounds —
//! for the CLI and the MCP tool alike (#1118).
//!
//! **Five passes, one ceiling, one total.** Shots, stills, narration, music
//! and sound effects are separate passes because they are separate models and vendors, but the
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

use scorsese_core::{AssetId, AssetKind, Project};

use crate::credentials::{Budget, CredentialError, Provider, resolve};
use crate::image::{self, GeminiProvider, ImageError, ImageProvider, Order};
use crate::music::{self, ElevenLabsComposer, MusicProvider};
use crate::sound::{self, ElevenLabsSounds, SoundProvider};
use crate::speech::{self, ElevenLabsProvider, SpeechProvider};
use crate::video::{self, GenerationError, VeoProvider, VideoProvider};

pub use super::done::Done;

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
    /// ElevenLabs Sound Effects, keyed (#1265).
    fn sound(&self) -> Result<Box<dyn SoundProvider>, CredentialError>;
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

    fn sound(&self) -> Result<Box<dyn SoundProvider>, CredentialError> {
        Ok(Box::new(ElevenLabsSounds::new(
            &resolve(Provider::ElevenLabs)?.secret,
        )))
    }

    fn rest(&self, wait: Duration) {
        std::thread::sleep(wait);
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
    /// The sound-effects pass.
    Sounds,
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
    if sound::pending(project, root) {
        let provider = vendors.sound().map_err(key(Step::Sounds))?;
        let budget = asked
            .budget
            .spend(done.shots.spent_cents + done.stills_spent() + done.voiced_spent());
        match sound::generate(project, root, &*provider, budget) {
            Ok(run) => done.sounds = run,
            Err(stopped) => {
                let says = stopped.says();
                done.sounds = stopped.run;
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
