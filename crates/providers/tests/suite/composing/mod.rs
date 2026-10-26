//! The generated-music lifecycle (#1264), driven against a composer that
//! costs nothing: a track composed, cached, refused and left alone by the
//! narration pass — and a line left alone by this one.

use crate::common;

mod ledger;
mod passes;

use std::cell::Cell;

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, MusicRequest, Project};
use scorsese_providers::credentials::Budget;
use scorsese_providers::music::{self, Brief, MusicProvider, Outcome, ProviderError};
use scorsese_providers::quote::generation;

/// Composes any track, counting the calls; `refusing` answers every one no.
#[derive(Default)]
struct Composer {
    calls: Cell<usize>,
    refusing: bool,
}

impl MusicProvider for Composer {
    fn compose(&self, _: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        if self.refusing {
            return Err(ProviderError::new("mock", "no music today"));
        }
        Ok(b"MP3".to_vec())
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// A sketched track of `seconds`, called `bed`.
fn track(seconds: Option<u32>) -> Asset {
    let mut asset = Asset::sketch(
        AssetId::new("bed"),
        AssetKind::GeneratedAudio,
        "calm lo-fi, soft piano",
    );
    asset.music = Some(MusicRequest {
        seconds,
        ..MusicRequest::default()
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
fn a_track_is_composed_priced_and_recorded() {
    let (dir, mut project) = holding("compose", vec![track(Some(40))]);
    let composer = Composer::default();
    let run = music::generate(&mut project, &dir, &composer, free()).expect("composed");
    let [(_, Outcome::Generated { path, .. })] = run.as_slice() else {
        panic!("{run:?}");
    };
    assert!(path.resolve(&dir).is_file());
    let asset = &project.assets[0];
    assert_eq!(asset.state, Some(GenerationState::Generated));
    assert_eq!(asset.estimated_cost_cents, Some(10));
    assert_eq!(run[0].1.spent_cents(), music::quote(40));
}

#[test]
fn a_track_already_composed_is_never_paid_for_again() {
    let (dir, mut project) = holding("compose-twice", vec![track(Some(40))]);
    let composer = Composer::default();
    music::generate(&mut project, &dir, &composer, free()).expect("first");
    let run = music::generate(&mut project, &dir, &composer, free()).expect("second");
    assert!(matches!(run[0].1, Outcome::Cached { .. }), "{run:?}");
    assert_eq!(composer.calls.get(), 1);
    assert!(!music::pending(&project, &dir));
}

#[test]
fn a_track_with_no_length_is_not_yet_and_sends_nothing() {
    let (dir, mut project) = holding("compose-untimed", vec![track(None)]);
    let composer = Composer::default();
    let run = music::generate(&mut project, &dir, &composer, free()).expect("a run");
    let [(_, Outcome::Incomplete { why })] = run.as_slice() else {
        panic!("{run:?}");
    };
    assert!(why.contains("music.seconds"), "{why}");
    assert_eq!(composer.calls.get(), 0);
}

#[test]
fn a_refusal_leaves_the_asset_as_it_was() {
    let (dir, mut project) = holding("compose-refused", vec![track(Some(40))]);
    let composer = Composer {
        refusing: true,
        ..Composer::default()
    };
    let run = music::generate(&mut project, &dir, &composer, free()).expect("a run");
    assert!(matches!(run[0].1, Outcome::Failed { .. }), "{run:?}");
    assert_eq!(project.assets[0].state, Some(GenerationState::Sketch));
}

#[test]
fn the_ceiling_holds_a_track() {
    let (dir, mut project) = holding("compose-ceiling", vec![track(Some(40))]);
    let composer = Composer::default();
    let stopped = music::generate(&mut project, &dir, &composer, Budget::new(9, 0))
        .expect_err("over the ceiling");
    assert!(
        stopped.says().contains("0 tracks were composed"),
        "{}",
        stopped.says()
    );
    assert_eq!(composer.calls.get(), 0);
}

#[test]
fn a_track_is_quoted_by_its_length() {
    let (dir, project) = holding("compose-quote", vec![track(Some(40))]);
    let quote = generation(&project, &dir).expect("quoted");
    let item = &quote.items[0];
    assert_eq!(item.charge.as_ref().map(|c| c.cents), Some(10));
    assert!(item.says.contains("40 seconds of music"), "{}", item.says);
}
