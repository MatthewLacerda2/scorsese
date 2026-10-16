//! A line and a track in one project: each pass takes its own, and the one
//! `generate` makes both under one ceiling.

use std::time::Duration;

use scorsese_core::{Asset, AssetId, AssetKind, SpeechRequest};
use scorsese_providers::credentials::{Budget, CredentialError};
use scorsese_providers::image::{ImageProvider, Order};
use scorsese_providers::music::{self, MusicProvider};
use scorsese_providers::passes::{self, Vendors};
use scorsese_providers::speech::{self, Brief, ProviderError, SpeechProvider, Spoken};
use scorsese_providers::video::VideoProvider;

use super::{Composer, holding, track};

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

/// A voice and a composer; nothing else is asked for.
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
        Ok(Box::new(Composer::default()))
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
fn each_pass_takes_only_its_own() {
    let (dir, mut project) = holding("compose-own", vec![line(), track(Some(40))]);
    let lines = speech::generate(&mut project, &dir, &Voice, Budget::new(100, 0)).expect("spoken");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].0, AssetId::new("vo"));
    let tracks = music::generate(
        &mut project,
        &dir,
        &Composer::default(),
        Budget::new(100, 0),
    )
    .expect("composed");
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].0, AssetId::new("bed"));
}

#[test]
fn one_generate_speaks_the_line_and_composes_the_track() {
    let (dir, mut project) = holding("compose-passes", vec![line(), track(Some(40))]);
    let asked = passes::Asked {
        budget: Budget::new(100, 0),
        patience: Duration::from_secs(60),
        order: Order::Now,
    };
    let (done, outcome) = passes::run(&mut project, &dir, &asked, &Fakes);
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(done.landed(), [AssetId::new("vo"), AssetId::new("bed")]);
    assert_eq!(done.spent_cents(), 1 + music::quote(40));
}
