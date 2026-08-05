//! `scorsese voices` — which voices there are, and whether one still works.
//!
//! The command that answers *what do I put in `voice_id`*. It spends nothing:
//! reading a list is free, and the answer is cached under the project's
//! `cache/` so browsing costs no round trip and works on a train.
//!
//! Two questions, one verb, because they are the same question at different
//! sizes: *which voices are there* and *is this one still there*. The second is
//! the one with a date on it — every ElevenLabs default voice expires on
//! 2026-12-31 — and it prints the reason rather than a status code.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::Project;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::{Answer, ElevenLabsVoices, Filters, builtin, library, require};

/// What was asked for on the command line.
pub(crate) struct Options {
    /// Search the Voice Library rather than the built-in set.
    pub(crate) library: bool,
    /// How to narrow the Voice Library. Ignored for the built-in set, which is
    /// small enough to read whole.
    pub(crate) filters: Filters,
    /// Read the vendor again even if the cache is current.
    pub(crate) refresh: bool,
    /// Ask about exactly one id instead of listing anything.
    pub(crate) check: Option<String>,
}

/// Lists the voices, or reports on the one that was asked about.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    // The project is opened before anything else because the cache lives inside
    // it: writing `cache/voices/` into a directory that is not a project would
    // leave a stray folder wherever somebody happened to be standing.
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;

    let key = resolve(Provider::ElevenLabs)?;
    let catalogue = ElevenLabsVoices::new(&key.secret);

    if let Some(voice_id) = &options.check {
        let voice = require(&catalogue, voice_id)?;
        println!("{}", voice.says());
        println!("\nStill a voice at ElevenLabs. It can be generated with.");
        return Ok(());
    }

    let answer = if options.library {
        library(project_dir, &catalogue, &options.filters, options.refresh)?
    } else {
        builtin(project_dir, &catalogue, options.refresh)?
    };
    report(&answer, options.library);
    Ok(())
}

/// Prints the list, then where it came from.
///
/// The provenance line comes out of the library rather than being written here,
/// so that this and the MCP tool cannot drift into describing the same cache
/// differently.
fn report(answer: &Answer, from_library: bool) {
    let what = if from_library {
        "the Voice Library"
    } else {
        "built-in"
    };
    if answer.voices.is_empty() {
        println!("No voices matched in {what}.");
    }
    for voice in &answer.voices {
        println!("{}", voice.says());
        if let Some(description) = voice
            .description
            .as_ref()
            .filter(|it| !it.trim().is_empty())
        {
            println!("    {}", description.trim());
        }
    }
    println!();
    println!("{}", answer.summary(what, "--refresh"));
}
