//! A line and a sound in one project: the narration pass leaves the sound
//! alone, and the one `generate` makes both under one ceiling.

use std::time::Duration;

use scorsese_core::{Asset, AssetId, AssetKind, SpeechRequest};
use scorsese_providers::credentials::{Budget, CredentialError};
use scorsese_providers::image::{ImageProvider, Order};
use scorsese_providers::music::MusicProvider;
use scorsese_providers::passes::{self, Vendors};
use scorsese_providers::sound::{self, SoundProvider};
use scorsese_providers::speech::{self, Brief, ProviderError, SpeechProvider, Spoken};
use scorsese_providers::video::VideoProvider;

use super::{Effects, effect, holding};

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

/// A voice and a sound maker; nothing else is asked for.
struct Fakes;

impl Vendors for Fakes {
    fn video(&self) -> Result<Box<dyn VideoProvider>, CredentialError> {
        unreachable!("no shot here")
    }

    fn image(&self) -> Result<Box<dyn ImageProvider>, CredentialError> {
        unreachable!("no still here")
    }

    fn speech(&self) -> Result<Box<dyn SpeechProvider>, CredentialError> {
        Ok(Box::new(Voice))
    }

    fn music(&self) -> Result<Box<dyn MusicProvider>, CredentialError> {
        unreachable!("no track here")
    }

    fn sound(&self) -> Result<Box<dyn SoundProvider>, CredentialError> {
        Ok(Box::new(Effects::default()))
    }

    fn rest(&self, _: Duration) {}
}

fn line() -> Asset {
    let mut line = Asset::sketch(
        AssetId::new("vo"),
        AssetKind::GeneratedAudio,
        "Hello there.",
    );
    line.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    line
}

#[test]
fn the_narration_pass_leaves_a_sound_alone() {
    let (dir, mut project) = holding("sound-own", vec![line(), effect(Some(2.0))]);
    let lines = speech::generate(&mut project, &dir, &Voice, Budget::new(100, 0)).expect("spoken");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].0, AssetId::new("vo"));
    let sounds = sound::generate(&mut project, &dir, &Effects::default(), Budget::new(100, 0))
        .expect("made");
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].0, AssetId::new("door"));
}

#[test]
fn one_generate_speaks_the_line_and_makes_the_sound() {
    let (dir, mut project) = holding("sound-passes", vec![line(), effect(Some(2.0))]);
    let asked = passes::Asked {
        budget: Budget::new(100, 0),
        patience: Duration::from_secs(60),
        order: Order::Now,
    };
    let (done, outcome) = passes::run(&mut project, &dir, &asked, &Fakes);
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(done.landed(), [AssetId::new("vo"), AssetId::new("door")]);
    assert_eq!(done.spent_cents(), 1 + sound::quote(2_000));
}
