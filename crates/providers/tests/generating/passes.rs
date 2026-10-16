//! The passes one `generate` makes, run against providers that cost nothing
//! (#1118): the ceiling carried from one pass into the next, a later pass
//! with no key keeping what an earlier one sent, and a collect that asks for
//! no key with nothing in flight.

use std::time::Duration;

use scorsese_core::{Asset, AssetId, AssetKind, Project, SpeechRequest};
use scorsese_providers::credentials::{Budget, CredentialError};
use scorsese_providers::image::{ImageProvider, Order};
use scorsese_providers::music::MusicProvider;
use scorsese_providers::passes::{self, Halt, Step, Vendors};
use scorsese_providers::quote::generation;
use scorsese_providers::speech::{Brief, ProviderError, SpeechProvider, Spoken};
use scorsese_providers::video::VideoProvider;

use super::mock::{Answer, Mock};
use super::sketched;

/// Speaks any line, for free.
struct Voice;

impl SpeechProvider for Voice {
    fn speak(&self, _: &Brief) -> Result<Spoken, ProviderError> {
        Ok(Spoken::from(b"MP3".to_vec()))
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// A shot that is ready on its first poll, a voice, and no Gemini key for
/// stills — `keyless` takes the voice's key away too.
struct Fakes {
    keyless: bool,
}

fn missing(provider: &'static str) -> CredentialError {
    CredentialError::Missing {
        provider,
        places: String::from("nowhere"),
    }
}

impl Vendors for Fakes {
    fn video(&self) -> Result<Box<dyn VideoProvider>, CredentialError> {
        Ok(Box::new(Mock::answering(
            "shot",
            vec![Answer::Ready(b"MP4".to_vec())],
        )))
    }

    fn image(&self) -> Result<Box<dyn ImageProvider>, CredentialError> {
        Err(missing("Gemini"))
    }

    fn speech(&self) -> Result<Box<dyn SpeechProvider>, CredentialError> {
        if self.keyless {
            return Err(missing("ElevenLabs"));
        }
        Ok(Box::new(Voice))
    }

    fn music(&self) -> Result<Box<dyn MusicProvider>, CredentialError> {
        Err(missing("ElevenLabs"))
    }

    fn rest(&self, _: Duration) {}
}

/// A sketched shot and a sketched line, and what each is quoted at.
fn shot_and_line(label: &str) -> (std::path::PathBuf, Project, u64, u64) {
    let (dir, mut project, _) = sketched(label, "a harbour at dawn");
    let mut line = Asset::sketch(
        AssetId::new("vo"),
        AssetKind::GeneratedAudio,
        "Hello there.",
    );
    line.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    project.assets.push(line);
    let quote = generation(&project, &dir).expect("quote");
    let cents = |id: &str| {
        let item = quote.items.iter().find(|item| item.subject == id);
        item.and_then(|item| item.charge.as_ref())
            .expect("charged")
            .cents
    };
    let (shot, line) = (cents("shot"), cents("vo"));
    (dir, project, shot, line)
}

fn asked(ceiling: u64) -> passes::Asked {
    passes::Asked {
        budget: Budget::new(ceiling, 0),
        patience: Duration::from_secs(60),
        order: Order::Now,
    }
}

#[test]
fn the_ceiling_counts_what_the_shots_spent_against_the_lines() {
    let (dir, mut project, shot, line) = shot_and_line("passes-ceiling");

    let (done, outcome) = passes::run(
        &mut project,
        &dir,
        &asked(shot + line - 1),
        &Fakes { keyless: false },
    );

    assert_eq!(done.shots.spent_cents, shot, "{done:?}");
    assert!(done.lines.is_empty(), "{done:?}");
    let Err(Halt::Stopped(says)) = outcome else {
        panic!("the line went over the ceiling: {outcome:?}");
    };
    assert!(says.contains("ceiling"), "{says}");
}

#[test]
fn a_run_that_fits_spends_on_every_pass_and_totals_them() {
    let (dir, mut project, shot, line) = shot_and_line("passes-fits");

    let (done, outcome) = passes::run(
        &mut project,
        &dir,
        &asked(shot + line),
        &Fakes { keyless: false },
    );

    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(done.spent_cents(), shot + line);
    assert_eq!(done.landed(), [AssetId::new("shot"), AssetId::new("vo")]);
}

#[test]
fn a_later_pass_with_no_key_keeps_what_the_shots_sent() {
    let (dir, mut project, shot, line) = shot_and_line("passes-keyless");

    let (done, outcome) = passes::run(
        &mut project,
        &dir,
        &asked(shot + line),
        &Fakes { keyless: true },
    );

    assert_eq!(done.shots.spent_cents, shot);
    assert!(
        matches!(
            outcome,
            Err(Halt::Key {
                step: Step::Lines,
                ..
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn collecting_with_nothing_in_flight_asks_for_no_key() {
    let (dir, mut project, _, _) = shot_and_line("passes-collect");

    let (done, outcome) = passes::collect(&mut project, &dir, &Fakes { keyless: true });

    assert!(outcome.is_ok() && done.is_empty(), "{outcome:?} {done:?}");
}
