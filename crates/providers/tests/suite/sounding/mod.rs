//! The generated-sound lifecycle (#1265), driven against a provider that
//! costs nothing: a sound made, cached, refused, billed for nothing, and left
//! alone by the narration pass.

use crate::common;

mod passes;

use std::cell::Cell;

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, Project, SoundRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::quote::generation;
use scorsese_providers::sound::{self, Brief, Outcome, ProviderError, SoundProvider};
use scorsese_providers::spending::history;

/// Makes any sound, counting the calls; `refusing` answers every one no, and
/// `billing` answers with nothing the vendor still charges for.
#[derive(Default)]
struct Effects {
    calls: Cell<usize>,
    refusing: bool,
    billing: bool,
}

impl SoundProvider for Effects {
    fn make(&self, _: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        if self.billing {
            return Err(ProviderError::billed("mock", "no audio came back"));
        }
        if self.refusing {
            return Err(ProviderError::new("mock", "no sounds today"));
        }
        Ok(b"MP3".to_vec())
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// A sketched sound of `seconds`, called `door`.
fn effect(seconds: Option<f64>) -> Asset {
    let mut asset = Asset::sketch(
        AssetId::new("door"),
        AssetKind::GeneratedAudio,
        "a heavy wooden door creaking open",
    );
    asset.sound = Some(SoundRequest {
        seconds,
        ..SoundRequest::default()
    });
    asset
}

/// A project holding `assets`, and its directory.
fn holding(label: &str, assets: Vec<Asset>) -> (std::path::PathBuf, Project) {
    let (dir, mut project) = common::project(label);
    project.assets.extend(assets);
    (dir, project)
}

fn free() -> Budget {
    Budget::new(10_000, 0)
}

#[test]
fn a_sound_is_made_priced_recorded_and_ledgered() {
    let (dir, mut project) = holding("sound", vec![effect(Some(2.5))]);
    let run = sound::generate(&mut project, &dir, &Effects::default(), free()).expect("made");
    let [(_, Outcome::Generated { path, .. })] = run.as_slice() else {
        panic!("{run:?}");
    };
    assert!(path.resolve(&dir).expect("a project path").is_file());
    let asset = &project.assets[0];
    assert_eq!(asset.state, Some(GenerationState::Generated));
    assert_eq!(asset.estimated_cost_cents, Some(1));
    assert_eq!(run[0].1.spent_cents(), sound::quote(2_500));
    assert_eq!(history(&project, &dir).len(), 1);
}

#[test]
fn a_sound_already_made_is_never_paid_for_again() {
    let (dir, mut project) = holding("sound-twice", vec![effect(Some(2.5))]);
    let effects = Effects::default();
    sound::generate(&mut project, &dir, &effects, free()).expect("first");
    let run = sound::generate(&mut project, &dir, &effects, free()).expect("second");
    assert!(matches!(run[0].1, Outcome::Cached { .. }), "{run:?}");
    assert_eq!(effects.calls.get(), 1);
    assert!(!sound::pending(&project, &dir));
}

#[test]
fn a_sound_with_no_length_is_not_yet_and_sends_nothing() {
    let (dir, mut project) = holding("sound-untimed", vec![effect(None)]);
    let effects = Effects::default();
    let run = sound::generate(&mut project, &dir, &effects, free()).expect("a run");
    let [(_, Outcome::Incomplete { why })] = run.as_slice() else {
        panic!("{run:?}");
    };
    assert!(why.contains("sound.seconds"), "{why}");
    assert_eq!(effects.calls.get(), 0);
}

#[test]
fn a_refusal_leaves_the_asset_as_it_was_and_costs_nothing() {
    let (dir, mut project) = holding("sound-refused", vec![effect(Some(2.5))]);
    let effects = Effects {
        refusing: true,
        ..Effects::default()
    };
    let run = sound::generate(&mut project, &dir, &effects, free()).expect("a run");
    assert!(matches!(run[0].1, Outcome::Failed { .. }), "{run:?}");
    assert_eq!(run[0].1.spent_cents(), 0);
    assert_eq!(project.assets[0].state, Some(GenerationState::Sketch));
    assert!(history(&project, &dir).is_empty());
}

#[test]
fn a_billed_empty_answer_is_charged_as_quoted() {
    let (dir, mut project) = holding("sound-billed", vec![effect(Some(2.5))]);
    let effects = Effects {
        billing: true,
        ..Effects::default()
    };
    let run = sound::generate(&mut project, &dir, &effects, free()).expect("a run");
    assert!(matches!(run[0].1, Outcome::Failed { .. }), "{run:?}");
    assert_eq!(run[0].1.spent_cents(), sound::quote(2_500));
    assert!(
        run[0].1.says().contains("billed as quoted"),
        "{}",
        run[0].1.says()
    );
    assert_eq!(project.assets[0].state, Some(GenerationState::Sketch));
    assert_eq!(
        history(&project, &dir).len(),
        1,
        "in the ledger all the same"
    );
}

#[test]
fn the_ceiling_holds_a_sound() {
    let (dir, mut project) = holding("sound-ceiling", vec![effect(Some(2.5))]);
    let effects = Effects::default();
    let stopped = sound::generate(&mut project, &dir, &effects, Budget::new(0, 0))
        .expect_err("over the ceiling");
    assert!(
        stopped.says().contains("0 sounds were made"),
        "{}",
        stopped.says()
    );
    assert_eq!(effects.calls.get(), 0);
}

#[test]
fn a_sound_is_quoted_by_its_length() {
    let (dir, project) = holding("sound-quote", vec![effect(Some(12.0))]);
    let quote = generation(&project, &dir).expect("quoted");
    let item = &quote.items[0];
    assert_eq!(item.charge.as_ref().map(|c| c.cents), Some(3));
    assert!(item.says.contains("12 seconds of sound"), "{}", item.says);
}
