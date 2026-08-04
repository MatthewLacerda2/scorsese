//! `scorsese settings` — where the keys are kept, and which of them resolved.

use anyhow::{Context, Result};
use scorsese_providers::credentials::{
    Environment, Provider, Settings, Source, resolve_from, settings_path,
};

/// Every provider, so the report is the same length whatever is configured —
/// a key that is missing has to appear in order to be seen to be missing.
const PROVIDERS: [Provider; 2] = [Provider::Gemini, Provider::ElevenLabs];

/// Prints where the settings file is, what resolved, and from where.
///
/// **No key is ever printed**, not truncated and not masked. A masked key is
/// still four characters of a credential in a terminal that scrolls into a log,
/// and the question anybody actually has is *did it find one, and from where*
/// — which is answerable without showing any of it.
pub(crate) fn run() -> Result<()> {
    let here = std::env::current_dir().context("reading the working directory")?;
    let environment = Environment::discover(&here);
    let settings = Settings::load().context("reading the settings file")?;

    match settings_path() {
        Ok(path) if path.is_file() => println!("settings file: {}", path.display()),
        Ok(path) => println!("settings file: {} (not created yet)", path.display()),
        Err(error) => println!("settings file: {error}"),
    }
    match environment.dotenv() {
        Some(path) => println!(".env:          {}", path.display()),
        None => println!(".env:          none found at or above the working directory"),
    }
    match settings.budget_cents {
        Some(cents) => println!("budget:        {} cents a run", cents),
        None => println!("budget:        no ceiling set"),
    }

    println!();
    for provider in PROVIDERS {
        let line = match resolve_from(provider, &environment, &settings) {
            Ok(found) => match found.source {
                Source::Environment => format!("found, from {}", provider.variable()),
                Source::Settings(path) => format!("found, from {}", path.display()),
            },
            Err(_) => String::from("missing"),
        };
        println!("{:<22} {line}", provider.label());
    }
    Ok(())
}
