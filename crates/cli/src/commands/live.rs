//! `scorsese check-providers` — every provider client against the real API.
//!
//! The logic is [`scorsese_providers::live`]'s; this is the terminal around
//! it: the plan printed as a quote, the same question `generate` asks, the
//! report, and the recording. It is the one verb that calls every vendor on
//! purpose, so it spends only after the quote, the question and the ceiling,
//! in that order — and the ceiling is asked again inside the run, where no
//! flag reaches it.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use scorsese_providers::credentials::{Budget, Environment, Provider, Settings};
use scorsese_providers::live::{self, Planned, Report, record};
use scorsese_providers::prices::dollars;

use super::confirm;

/// What was asked for on the command line.
#[derive(Debug)]
pub(crate) struct Options {
    /// Pay for one real Veo shot.
    pub(crate) include_veo: bool,
    /// Seconds to watch it for.
    pub(crate) veo_wait: u64,
    /// Quote and stop.
    pub(crate) dry_run: bool,
    /// Answer the question in advance.
    pub(crate) yes: bool,
    /// Where to save what came back.
    pub(crate) record: Option<PathBuf>,
}

/// Quotes, asks, runs, reports — and fails when a provider is broken.
pub(crate) fn run(options: &Options) -> Result<()> {
    let here = std::env::current_dir().context("reading the working directory")?;
    let settings = Settings::load().context("reading the settings file")?;
    let check = live::Options {
        include_veo: options.include_veo,
        veo_patience: Duration::from_secs(options.veo_wait),
    };
    let plan = live::plan(&check, &Environment::discover(&here), &settings);
    quote(&plan);

    let budget = Budget::from_settings(&settings, 0);
    live::permit(&plan, budget)?;
    if options.dry_run {
        println!("Nothing was sent.");
        return Ok(());
    }
    if live::total(&plan) > 0 && !permitted(options.yes)? {
        println!("Nothing was sent.");
        return Ok(());
    }

    println!();
    let reports = live::run(&plan, &check, budget, &mut |line| println!("{line}"))?;
    reported(&reports);
    if let Some(dir) = &options.record {
        recorded(dir, &reports)?;
    }
    let broken = reports
        .iter()
        .filter(|report| report.verdict().is_failure())
        .count();
    if broken > 0 {
        bail!("{broken} provider(s) failed the live check — see above");
    }
    Ok(())
}

/// Whether to go ahead, asked the way `generate` asks.
fn permitted(yes: bool) -> Result<bool> {
    match confirm::verdict(yes, confirm::interactive()) {
        confirm::Verdict::Ahead => Ok(true),
        confirm::Verdict::NobodyThere => bail!(confirm::nobody_there("check-providers")),
        confirm::Verdict::Ask => confirm::asked("Make these calls?"),
    }
}

/// The plan, as a quote.
fn quote(plan: &[Planned]) {
    println!("The live provider check will call:");
    for planned in plan {
        println!();
        println!("{}", planned.provider.label());
        if let Some(why) = &planned.skipped {
            println!("  skipped: {why}");
        }
        for call in &planned.calls {
            println!("  - {call}");
        }
    }
    println!();
    println!(
        "At most {} in all — our arithmetic over the published rates, never a bill.",
        dollars(live::total(plan))
    );
}

/// What each provider did.
fn reported(reports: &[Report]) {
    for report in reports {
        println!();
        println!("{}: {}", report.provider.label(), report.verdict().word());
        for step in &report.steps {
            let detail = step.verdict.detail();
            let dash = if detail.is_empty() { "" } else { " — " };
            println!("  {:<14}{}{dash}{detail}", step.verdict.word(), step.call);
            for note in &step.notes {
                println!("  {:<14}  {note}", "");
            }
        }
    }
    let cents: u64 = reports.iter().map(|report| report.cents).sum();
    println!();
    println!(
        "About {} spent — our arithmetic, never a bill.",
        dollars(cents)
    );
}

/// Writes every reply into `dir`, and says what it wrote.
fn recorded(dir: &std::path::Path, reports: &[Report]) -> Result<()> {
    println!();
    println!(
        "Recorded into {} — scrubbed of keys, but read before committing:",
        dir.display()
    );
    for report in reports {
        let written = record::write(dir, slug(report.provider), &report.exchanges)
            .with_context(|| format!("writing into {}", dir.display()))?;
        for line in written {
            println!("  {line}");
        }
    }
    Ok(())
}

/// A provider as a file name starts: `gemini`, `elevenlabs`, `anthropic`,
/// `pixabay`.
fn slug(provider: Provider) -> &'static str {
    match provider {
        Provider::Gemini => "gemini",
        Provider::ElevenLabs => "elevenlabs",
        Provider::Anthropic => "anthropic",
        Provider::Pixabay => "pixabay",
    }
}
