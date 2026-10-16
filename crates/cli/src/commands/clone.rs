//! `scorsese voice-clone` — a voice from the project's own recordings
//! (#1263): *narrate in my own voice*.
//!
//! Three things it can be asked, as `voice-design` has four: clone, list what
//! this project cloned, and forget a cloned voice. None spends anything —
//! creating an Instant Voice Clone is not billed — but cloning needs
//! `--consent`, given only once the voice is the user's own or its owner said
//! yes, and nothing is read or sent without it.

use std::path::Path;

use anyhow::Result;
use scorsese_core::AssetId;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::clone::{
    Ask, FORGOTTEN, LEDGER_FILE, check, clone, cloned, forget,
};
use scorsese_providers::voices::design::ElevenLabsStudio;

/// What was asked for on the command line.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// An imported audio asset to clone from; repeat for more. About one
    /// to two minutes of clear speech in all is what ElevenLabs
    /// recommends, and more than three adds little.
    #[arg(long = "asset", value_name = "ASSET_ID", conflicts_with_all = ["list", "forget"])]
    pub(crate) assets: Vec<String>,
    /// What to call the voice in your ElevenLabs account — say whose it is.
    #[arg(long, conflicts_with_all = ["list", "forget"])]
    pub(crate) name: Option<String>,
    /// What the voice is, in a sentence, kept with it at the vendor.
    #[arg(long, requires = "name")]
    pub(crate) description: Option<String>,
    /// Ask ElevenLabs to clean background noise out of the recordings
    /// first. Only for recordings with some: on clean ones it can lower
    /// the quality.
    #[arg(long, requires = "name")]
    pub(crate) remove_background_noise: bool,
    /// The voice is your own, or its owner has given you permission to
    /// clone it. Nothing is read or sent without it.
    #[arg(long, requires = "name")]
    pub(crate) consent: bool,
    /// List the voices this project has cloned, with the recordings each
    /// was made from. Touches no network.
    #[arg(long, conflicts_with_all = ["forget"])]
    pub(crate) list: bool,
    /// Delete this cloned voice from your ElevenLabs account, by the
    /// voice_id --list shows, freeing the voice slot it takes. The
    /// recordings stay in the project.
    #[arg(long, value_name = "VOICE_ID")]
    pub(crate) forget: Option<String>,
}

/// Clones a voice, forgets one, or reads the ledger.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let project = super::open(project_dir)?;

    if options.list {
        ledger(project_dir);
        return Ok(());
    }
    if let Some(voice) = &options.forget {
        let forgotten = forget(project_dir, &studio()?, voice)?;
        println!("{}\n\n{FORGOTTEN}", forgotten.says());
        return Ok(());
    }

    let ask = Ask {
        assets: options.assets.iter().map(AssetId::new).collect(),
        name: options.name.clone().unwrap_or_default(),
        description: options.description.clone(),
        remove_background_noise: options.remove_background_noise,
        consent: options.consent,
    };
    // Before the key, so the consent sentence is the first thing said.
    check(ask.consent, &ask.name)?;
    let done = clone(project_dir, &project, &studio()?, &ask)?;
    println!("{}", done.voice.says());
    println!();
    println!(
        "That voice now exists in your ElevenLabs account, which is not inside this project — \
         the id above travels with an `scp -r` and the voice itself does not. The recordings \
         it was cloned from are written into {LEDGER_FILE}, so a lost voice can be cloned \
         again from them. It cost nothing, and takes one of your account's custom voice \
         slots until you forget it."
    );
    if done.cloned.requires_verification {
        println!(
            "ElevenLabs says this voice needs verifying before it narrates: open it among your \
             voices at elevenlabs.io and follow its steps."
        );
    }
    if let Some(advice) = done.advice {
        println!("{advice}");
    }
    Ok(())
}

/// The studio, keyed from the one resolver.
fn studio() -> Result<ElevenLabsStudio> {
    Ok(ElevenLabsStudio::new(
        &resolve(Provider::ElevenLabs)?.secret,
    ))
}

/// What this project has cloned.
fn ledger(project_dir: &Path) {
    let entries = cloned(project_dir);
    if entries.is_empty() {
        println!(
            "No voices have been cloned in this project. `scorsese voice-clone --asset <ID> \
             --name \"<whose voice>\" --consent` clones one from its recordings."
        );
    }
    for entry in &entries {
        println!("{}", entry.says());
    }
}
