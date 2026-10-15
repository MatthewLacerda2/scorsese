use std::path::PathBuf;

use super::Quote;
use scorsese_core::{
    Asset, AssetId, AssetKind, Fps, GenerationState, Project, ProjectPath, SpeechRequest,
};
use scorsese_providers::credentials::Settings;
use scorsese_providers::voices::design::LEDGER_FILE;

/// A machine nobody has configured: no keys, and no ceiling.
///
/// Stated rather than loaded, so these assertions are about the arithmetic
/// and not about whoever ran them.
fn fresh() -> Settings {
    Settings::default()
}

/// An empty project directory of its own, so nothing is already in its
/// `generated/` and no ledger sits beside it unless a test writes one.
fn root(label: &str) -> PathBuf {
    let dir = crate::scratch::fresh(&format!("scorsese-app-quote-{label}"), ".scor");
    std::fs::create_dir_all(dir.join("generated")).expect("create the project directory");
    dir
}

/// A project with one shot to make and one line of narration.
fn project() -> Project {
    let mut project = Project::new("quoting", Fps::THIRTY);
    project.assets.push(Asset::sketch(
        AssetId::new("shot"),
        AssetKind::GeneratedVideo,
        "a city at dawn",
    ));
    let mut line = Asset::sketch(
        AssetId::new("vo"),
        AssetKind::GeneratedAudio,
        "In a city that never sleeps.",
    );
    line.speech = Some(SpeechRequest {
        voice_id: Some(String::from("a-voice")),
        ..SpeechRequest::default()
    });
    project.assets.push(line);
    project
}

/// What the quote charges for one kind, by asset.
fn charged(quote: &Quote, project: &Project, kind: AssetKind) -> Vec<(String, u64)> {
    quote
        .of_kind(project, kind)
        .into_iter()
        .filter_map(|item| Some((item.subject.clone(), item.charge.as_ref()?.cents)))
        .collect()
}

/// The bug this was found with, and the reason it is worth a test: a
/// narration prompt is a prompt, so quoting everything "prompted" priced a
/// line of speech as an eight-second video shot — and doubled the total a
/// person was about to agree to. The shot is grouped with the shots, and the
/// line is quoted through the speech table at a cent rather than through the
/// video table at ninety-six.
#[test]
fn narration_is_not_quoted_as_a_video_shot() {
    let project = project();
    let quote = Quote::of(&project, &root("narration"), &fresh());
    let shots = charged(&quote, &project, AssetKind::GeneratedVideo);
    assert_eq!(shots, [(String::from("shot"), 96)], "8s of Fast at 1080p");
    let lines = charged(&quote, &project, AssetKind::GeneratedAudio);
    assert_eq!(
        lines,
        [(String::from("vo"), 1)],
        "28 characters, never the video rate"
    );
    assert_eq!(quote.total_cents(), 97);
}

/// A line nobody has chosen a voice for is listed, charges nothing, and says
/// why in the run's own words. Dropping it would leave a person wondering
/// which of their narrations the total covered.
#[test]
fn a_line_that_is_not_ready_is_listed_free_and_says_why() {
    let mut project = project();
    project.assets[1].speech = None;

    let quote = Quote::of(&project, &root("unready"), &fresh());
    let lines = quote.of_kind(&project, AssetKind::GeneratedAudio);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].charge.is_none());
    assert!(lines[0].says.contains("no voice"), "{:?}", lines[0].says);
    assert_eq!(quote.total_cents(), 96);
}

/// #1104: a shot whose `state` still says `generated` but whose prompt was
/// edited since is handed over and paid for by the run, which decides by the
/// brief hash. The quote used to read the `state` and price it at nothing.
#[test]
fn an_edited_generated_shot_is_quoted_at_its_price() {
    let mut project = project();
    let shot = &mut project.assets[0];
    shot.state = Some(GenerationState::Generated);
    shot.path = Some(ProjectPath::new("generated/before-the-edit.mp4"));
    shot.estimated_cost_cents = Some(96);
    shot.prompt = Some(String::from("a city at dusk"));

    let quote = Quote::of(&project, &root("edited"), &fresh());
    let shots = charged(&quote, &project, AssetKind::GeneratedVideo);
    assert_eq!(shots, [(String::from("shot"), 96)]);
    assert_eq!(quote.spent_cents, 96, "the first generation is still spent");
}

/// What a designed voice cost is spent money, so the ceiling counts it — the
/// window's own sum of the assets once left it out.
#[test]
fn designed_voices_count_against_the_ceiling() {
    let root = root("designed");
    let ledger = serde_json::json!([{
        "voice_id": "v1", "name": "Narrator", "prompt": "a warm voice",
        "passage": "Hello there.", "provider": "ElevenLabs",
        "design": "abc", "estimated_cost_cents": 30
    }]);
    std::fs::write(root.join(LEDGER_FILE), ledger.to_string()).expect("write the ledger");

    let settings = Settings {
        budget_cents: Some(120),
        ..Settings::default()
    };
    let quote = Quote::of(&project(), &root, &settings);
    assert_eq!(quote.spent_cents, 30);
    assert!(quote.over_budget(), "30c spent and 97c more crosses 120c");
}

/// No ceiling refuses nothing — a limit nobody set is not a limit.
#[test]
fn a_run_with_no_ceiling_is_never_over_budget() {
    assert!(!Quote::of(&project(), &root("unlimited"), &fresh()).over_budget());
}

/// And a ceiling the run would cross is read off the settings it was handed.
#[test]
fn a_ceiling_the_run_would_cross_is_refused() {
    let settings = Settings {
        budget_cents: Some(50),
        ..Settings::default()
    };
    let quote = Quote::of(&project(), &root("ceiling"), &settings);
    assert_eq!(quote.total_cents(), 97, "97c against a 50c ceiling");
    assert!(quote.over_budget());
}
