//! The money guarantee: an unchanged line is never paid for twice.

use scorsese_providers::credentials::Budget;
use scorsese_providers::speech::{Outcome, generate};

use crate::mock::{Answer, Mock};
use crate::{asset_mut, sketched};

/// Three whole runs, one request. The rest find the file and stop.
#[test]
fn a_line_already_on_disk_is_never_spoken_again() {
    let (dir, mut project, _) = sketched("cached", "the opening line");
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let again = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(again[0].1, Outcome::Cached { .. }),
        "{:?}",
        again[0].1
    );
    assert_eq!(provider.requests(), 1, "the cache did not hold");
    assert_eq!(again[0].1.spent_cents(), 0, "a cache hit costs nothing");
    std::fs::remove_dir_all(&dir).ok();
}

/// Rewriting the line is a different brief, so it is a different file and a
/// second charge. That is `stale` arriving by arithmetic rather than by anyone
/// remembering to mark it.
#[test]
fn rewriting_the_line_is_a_different_brief() {
    let (dir, mut project, id) = sketched("rewritten", "the first take");
    let provider = Mock::giving(vec![
        Answer::Speaks(b"ONE".to_vec()),
        Answer::Speaks(b"TWO".to_vec()),
    ]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let first = project.asset(&id).and_then(|a| a.path.clone());

    asset_mut(&mut project, &id).prompt = Some(String::from("the second take"));
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let second = project.asset(&id).and_then(|a| a.path.clone());

    assert_ne!(first, second, "a rewritten line is a different file");
    assert_eq!(provider.requests(), 2, "it had to be spoken again");
    std::fs::remove_dir_all(&dir).ok();
}

/// Changing the voice changes the audio, so it has to change the fingerprint.
/// Hashing only the words would serve a line back in the wrong voice for ever.
#[test]
fn changing_the_voice_is_a_different_brief() {
    let (dir, mut project, id) = sketched("revoiced", "the same words throughout");
    let provider = Mock::willing();

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let first = project.asset(&id).and_then(|a| a.path.clone());

    let mut request = project.asset(&id).expect("the asset").speech_request();
    request.voice_id = Some(String::from("another-voice"));
    asset_mut(&mut project, &id).speech = Some(request);
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_ne!(
        first,
        project.asset(&id).and_then(|a| a.path.clone()),
        "another voice is another recording"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Every other field the brief carries has to move the fingerprint too — none
/// of them is decoration, and each changes the audio that comes back.
#[test]
fn the_model_the_language_and_the_seed_all_move_the_fingerprint() {
    let (dir, mut project, id) = sketched("fields", "one line, several requests");
    let provider = Mock::willing();
    let mut seen = std::collections::HashSet::new();

    for change in [0, 1, 2, 3] {
        let mut request = project.asset(&id).expect("the asset").speech_request();
        match change {
            // Any model but the default, or this case is the baseline again.
            1 => request.model = scorsese_core::SpeechModel::Expressive,
            2 => request.language = Some(String::from("pt")),
            3 => request.seed = Some(7),
            _ => {}
        }
        asset_mut(&mut project, &id).speech = Some(request);
        generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
        seen.insert(project.asset(&id).and_then(|a| a.path.clone()));
    }

    assert_eq!(seen.len(), 4, "each field must produce its own recording");
    std::fs::remove_dir_all(&dir).ok();
}
