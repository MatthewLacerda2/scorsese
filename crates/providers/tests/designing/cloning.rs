//! Cloning a voice from a project's own recordings (#1263): consent first,
//! only imported audio, and what made the voice written down beside it.

use scorsese_core::{Asset, AssetId, AssetKind, MediaMetadata, Project, ProjectPath};
use scorsese_providers::voices::clone::{Ask, CONSENT, TERMS, USE_POLICY, clone, cloned, forget};

use super::common::{project, write};
use super::fake::Fake;

/// A project holding one recording of `seconds`, `take`, at `assets/take.mp3`.
fn recorded(label: &str, seconds: f64) -> (std::path::PathBuf, Project) {
    let (dir, mut project) = project(label);
    write(&dir, "assets/take.mp3", "ID3 a recording");
    let mut asset = Asset::imported(
        AssetId::new("take"),
        AssetKind::Audio,
        ProjectPath::new("assets/take.mp3"),
    );
    asset.sha256 = Some("ab".repeat(32));
    asset.media = Some(MediaMetadata {
        duration_seconds: Some(seconds),
        ..MediaMetadata::default()
    });
    project.assets.push(asset);
    project
        .assets
        .push(Asset::text(AssetId::new("title"), "Hello"));
    (dir, project)
}

fn ask(assets: &[&str], consent: bool) -> Ask {
    Ask {
        assets: assets.iter().map(|id| AssetId::new(*id)).collect(),
        name: String::from("My voice"),
        description: Some(String::from("me, reading")),
        remove_background_noise: false,
        consent,
    }
}

#[test]
fn without_consent_nothing_is_read_or_sent() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-no-consent", 90.0);

    let error = clone(&dir, &project, &studio, &ask(&["take"], false)).expect_err("no consent");

    let said = error.to_string();
    assert!(said.contains(CONSENT), "{said}");
    assert!(studio.cloned.borrow().is_empty());
    assert!(cloned(&dir).is_empty());
}

#[test]
fn consent_names_the_vendors_terms() {
    assert!(
        CONSENT.contains(TERMS) && CONSENT.contains(USE_POLICY),
        "{CONSENT}"
    );
    assert!(CONSENT.contains("your own voice"), "{CONSENT}");
}

#[test]
fn a_clone_sends_the_recordings_and_records_what_made_it() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-made", 90.0);

    let done = clone(&dir, &project, &studio, &ask(&["take"], true)).expect("the clone");

    let sent = studio.cloned.borrow();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "My voice");
    assert_eq!(sent[0].recordings[0].file_name, "take.mp3");
    assert_eq!(sent[0].recordings[0].media_type, "audio/mpeg");
    assert_eq!(sent[0].recordings[0].bytes, b"ID3 a recording");
    assert_eq!(
        done.advice, None,
        "ninety seconds is what the vendor recommends"
    );
    let ledger = cloned(&dir);
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].voice_id, done.voice.id);
    assert_eq!(ledger[0].recordings[0].asset.as_str(), "take");
    assert_eq!(
        ledger[0].recordings[0].sha256.as_deref(),
        Some("ab".repeat(32).as_str())
    );
    assert!(
        ledger[0].says().contains("from take"),
        "{}",
        ledger[0].says()
    );
}

#[test]
fn a_short_recording_clones_and_says_what_the_vendor_recommends() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-short", 15.0);

    let done = clone(&dir, &project, &studio, &ask(&["take"], true)).expect("the clone");

    assert!(
        done.advice
            .is_some_and(|said| said.contains("one to two minutes"))
    );
}

#[test]
fn only_an_imported_recording_is_cloned_from() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-not-audio", 90.0);

    let error = clone(&dir, &project, &studio, &ask(&["title"], true)).expect_err("text");
    assert!(
        error.to_string().contains("not an imported audio"),
        "{error}"
    );
    let error = clone(&dir, &project, &studio, &ask(&["nobody"], true)).expect_err("absent");
    assert!(error.to_string().contains("no asset nobody"), "{error}");
    let error = clone(&dir, &project, &studio, &ask(&[], true)).expect_err("none");
    assert!(error.to_string().contains("at least one"), "{error}");
    assert!(studio.cloned.borrow().is_empty());
}

#[test]
fn a_free_plan_is_said_as_the_plan_and_records_nothing() {
    let mut studio = Fake::offering(&[]);
    studio.free_plan = true;
    let (dir, project) = recorded("clone-free", 90.0);

    let error = clone(&dir, &project, &studio, &ask(&["take"], true)).expect_err("free plan");

    assert!(error.to_string().contains("Starter"), "{error}");
    assert!(cloned(&dir).is_empty());
}

#[test]
fn a_cloned_voice_is_forgotten_at_the_vendor_and_marked() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-forget", 90.0);
    let voice = clone(&dir, &project, &studio, &ask(&["take"], true))
        .expect("the clone")
        .voice
        .id;

    forget(&dir, &studio, "someone-elses").expect_err("not ours");
    let forgotten = forget(&dir, &studio, &voice).expect("forgetting it");

    assert!(forgotten.forgotten);
    assert_eq!(*studio.forgotten.borrow(), vec![voice.clone()]);
    assert!(!cloned(&dir)[0].kept());
    forget(&dir, &studio, &voice).expect_err("it is no longer kept");
}
