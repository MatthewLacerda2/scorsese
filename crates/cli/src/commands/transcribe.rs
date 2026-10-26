//! `scorsese transcribe`
//!
//! An imported recording's speech heard by the local Whisper server, its
//! words kept beside it (#1549) — and, with `--fix`, a misheard word
//! corrected in place, keeping its times. The same work as the
//! `transcribe` and `transcript_fix` tools.

use std::path::Path;

use anyhow::{Result, anyhow};
use scorsese_core::AssetId;
use scorsese_core::words::Fix;
use scorsese_providers::transcribe::{
    self, Asked, Hearing, LONGEST, Language, Undecoded, Whisper, correct,
};
use scorsese_render::audio::speech;
use scorsese_render::{Cancel, Tools};

/// What `scorsese transcribe` takes.
#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// The imported video or sound whose speech to hear.
    asset: String,
    /// The language spoken, as a code like `pt` or `en`. Detected when not
    /// given.
    #[arg(long, value_parser = parse_language)]
    language: Option<Language>,
    /// Hear it afresh even though a transcript is kept, replacing it and every
    /// correction made to it.
    #[arg(long, conflicts_with = "fix")]
    again: bool,
    /// Correct word NUMBER, as the transcript was last listed, to TEXT; an
    /// empty TEXT drops it. Repeatable. Nothing is heard: the kept transcript
    /// is corrected.
    #[arg(long, value_name = "NUMBER=TEXT", value_parser = parse_fix)]
    fix: Vec<Fix>,
}

/// Hears the asset, or corrects its transcript, and lists the words.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let project = super::open(project_dir)?;
    let asset = AssetId::new(options.asset.as_str());
    if !options.fix.is_empty() {
        let corrected = correct(&project, project_dir, &asset, &options.fix)?;
        println!("{}", corrected.said());
        return Ok(());
    }
    let asked = Asked {
        language: options.language.clone(),
        again: options.again,
    };
    let whisper = Whisper::new(&transcribe::address()?);
    let hearing = Hearing {
        asked: &asked,
        transcriber: &whisper,
        decode: |file: &Path| {
            let failed = |error: &dyn std::fmt::Display| Undecoded::Failed(error.to_string());
            let tools = Tools::discover().map_err(|error| failed(&error))?;
            speech(&tools, file, LONGEST.as_secs_f64(), &Cancel::new()).map_err(|e| failed(&e))
        },
        // A person at a terminal stops it with Ctrl-C.
        stop: &|| false,
    };
    let done = transcribe::transcribe(&project, project_dir, &asset, hearing)?;
    println!("{}", done.said());
    Ok(())
}

fn parse_language(text: &str) -> Result<Language> {
    Ok(text.parse()?)
}

fn parse_fix(text: &str) -> Result<Fix> {
    let (word, said) = text
        .split_once('=')
        .ok_or_else(|| anyhow!("`{text}` is not NUMBER=TEXT, like `3=Kassab`"))?;
    let word = word
        .trim()
        .parse()
        .map_err(|_| anyhow!("`{word}` is not a word's number, in NUMBER=TEXT"))?;
    Ok(Fix {
        word,
        text: said.to_owned(),
    })
}
