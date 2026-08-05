//! `scorsese voice-design` — a voice from a sentence.
//!
//! The second command in the binary that spends money, and the first that
//! leaves something behind **outside** the project directory. Both facts shape
//! everything it prints.
//!
//! Three things it can be asked, and they are one verb because they are one
//! subject at three moments: design three candidates, keep the one that was
//! chosen, and read back what this project has designed. Only the first spends
//! anything; keeping was paid for when the candidate was designed, and reading
//! the ledger touches no network at all.
//!
//! What this command deliberately does **not** do is play the samples. The
//! audition is a window's job — three files and a pair of ears — and this
//! prints where they are so that anything at all can play them.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::Project;
use scorsese_providers::credentials::{Budget, Provider, Settings, resolve};
use scorsese_providers::prices::dollars;
use scorsese_providers::spending;
use scorsese_providers::voices::design::{
    Brief, Designing, ElevenLabsStudio, Kept, LEDGER_FILE, design, designed, estimate, keep,
};

/// What was asked for on the command line.
pub(crate) struct Options {
    /// What the voice should be like, in a sentence.
    pub(crate) prompt: Option<String>,
    /// What the candidates read aloud, and the only thing billed.
    pub(crate) passage: Option<String>,
    /// The vendor's best-effort determinism knob.
    pub(crate) seed: Option<u32>,
    /// How literally the description should be followed.
    pub(crate) guidance: Option<f64>,
    /// Say what it would cost and stop.
    pub(crate) dry_run: bool,
    /// Keep this candidate, making it a real voice in the account.
    pub(crate) keep: Option<String>,
    /// What to call the kept voice.
    pub(crate) name: Option<String>,
    /// Read back what this project has already designed.
    pub(crate) list: bool,
}

/// Designs a voice, keeps one, or reads the ledger.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    // The project is opened before anything else because everything this writes
    // lands inside it — samples in `generated/`, the ledger beside
    // `project.json` — and writing either into a directory that is not a
    // project would leave a stray folder wherever somebody happened to stand.
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    if options.list {
        ledger(project_dir);
        return Ok(());
    }
    if let Some(chosen) = &options.keep {
        return kept(project_dir, chosen, options.name.as_deref());
    }

    let brief = brief(options)?;
    if options.dry_run {
        // No key, no call, nothing sent. The quote is arithmetic over the
        // passage and needs nothing else.
        println!("{}", estimate(&brief.passage)?.says());
        return Ok(());
    }

    let key = resolve(Provider::ElevenLabs)?;
    let studio = ElevenLabsStudio::new(&key.secret);
    let budget = Budget::from_settings(
        &Settings::load().unwrap_or_default(),
        spending::so_far(&project, project_dir).total(),
    );
    report(&design(project_dir, &studio, &brief, budget)?);
    Ok(())
}

/// The brief the options describe, refused here if the vendor would refuse it.
fn brief(options: &Options) -> Result<Brief> {
    let prompt = options.prompt.as_deref().unwrap_or_default();
    let passage = options.passage.as_deref().unwrap_or_default();
    Ok(Brief::new(prompt, passage, options.seed, options.guidance)?)
}

/// The three candidates, where they are, and what they cost.
fn report(designing: &Designing) {
    for (index, sample) in designing.session.candidates.iter().enumerate() {
        println!("{}. {}", index + 1, sample.generated_voice_id);
        println!("   {}", sample.path);
    }
    println!();
    if designing.already_paid {
        println!(
            "Already designed — these three were paid for before and are on disk, so this \
             cost nothing."
        );
    } else {
        println!("{}", designing.estimate.says());
    }
    println!(
        "Listen, then keep the one you want:\n  \
         scorsese voice-design --keep <ID> --name \"<what to call it>\""
    );
}

/// Turns one candidate into a voice, and says what was written down.
fn kept(project_dir: &Path, chosen: &str, name: Option<&str>) -> Result<()> {
    let name = name.filter(|name| !name.trim().is_empty()).context(
        "a designed voice needs a name — it is what the voice will be called in your \
         ElevenLabs account, and the only thing you will recognise it by there",
    )?;
    let key = resolve(Provider::ElevenLabs)?;
    let studio = ElevenLabsStudio::new(&key.secret);
    let Kept { voice, designed } = keep(project_dir, &studio, chosen, name)?;

    println!("{}", voice.says());
    println!();
    println!(
        "That voice now exists in your ElevenLabs account, which is **not** inside this \
         project — the id above travels with an `scp -r` and the voice itself does not. \
         So the description and the seed that made it are written into {LEDGER_FILE}, and \
         if the voice is ever lost that file is what lets you ask for it again."
    );
    if designed.seed.is_none() {
        println!(
            "Designed without a seed, so asking again will not get as close. Pass --seed \
             next time if a voice is one you would want back."
        );
    }
    Ok(())
}

/// What this project has designed, and what it came to.
fn ledger(project_dir: &Path) {
    let entries = designed(project_dir);
    if entries.is_empty() {
        println!(
            "No voices have been designed in this project. `scorsese voice-design --prompt \
             ... --text ...` designs three candidates to choose between."
        );
        return;
    }
    for entry in &entries {
        println!("{}", entry.says());
    }
    println!();
    println!(
        "{} designed here, about {} spent designing them — our arithmetic over the \
         published rate, never a bill, and counted apart from what the shots cost.",
        entries.len(),
        dollars(scorsese_providers::voices::design::spent(project_dir))
    );
}
