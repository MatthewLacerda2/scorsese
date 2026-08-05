//! The money guarantee: an unchanged brief is never paid for twice.

use scorsese_core::{Asset, AssetId, AssetKind, ProjectPath, VideoModel, VideoRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Brief, Outcome, generate};

use crate::mock::{Answer, Mock};
use crate::sketched;

/// Two whole runs, one submission. The second run finds the file and stops.
#[test]
fn a_brief_already_on_disk_is_never_sent_again() {
    let (dir, mut project, _) = sketched("cached", "a red balloon at dusk");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"MP4".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let again = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        matches!(again[0].1, Outcome::Cached { .. }),
        "{:?}",
        again[0].1
    );
    assert_eq!(provider.submissions(), 1, "the cache did not hold");
    assert_eq!(again[0].1.spent_cents(), 0, "a cache hit costs nothing");
}

/// Editing the prompt is a different brief, so it is a different file and a
/// second generation. That is the sketch lifecycle's `stale` arriving by
/// arithmetic rather than by anyone remembering to mark it.
///
/// The old video stays on the asset until the new one is collected, which is
/// deliberate: a re-prompted shot still renders as what it currently is rather
/// than reverting to a slug card while the replacement is in flight.
#[test]
fn editing_the_prompt_is_a_different_brief() {
    let (dir, mut project, id) = sketched("edited", "a red balloon");
    let provider = Mock::answering(
        "shot",
        vec![
            Answer::Ready(b"ONE".to_vec()),
            Answer::Ready(b"TWO".to_vec()),
        ],
    );
    let run = |project: &mut scorsese_core::Project| {
        generate(project, &dir, &provider, Budget::unlimited(0)).expect("a run")
    };

    run(&mut project);
    run(&mut project);
    let first = project.asset(&id).and_then(|a| a.path.clone());
    assert!(first.is_some(), "the first generation landed somewhere");

    let asset = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == id)
        .expect("the asset");
    asset.prompt = Some(String::from("a blue balloon"));
    asset.state = Some(scorsese_core::GenerationState::Stale);

    let queued = run(&mut project);
    assert!(
        matches!(queued[0].1, Outcome::Queued { .. }),
        "{:?}",
        queued[0].1
    );
    assert_eq!(provider.submissions(), 2, "an edited prompt is not cached");
    assert_eq!(
        project.asset(&id).and_then(|a| a.path.clone()),
        first,
        "the shot still shows what it currently is while the new one generates"
    );

    run(&mut project);
    assert_ne!(
        project.asset(&id).and_then(|a| a.path.clone()),
        first,
        "and points at the new video once that one is collected"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The case a file-name hash gets wrong. `face.png` is replaced by a different
/// picture of the same name: same path, same brief on paper, different image —
/// and it has to be a different fingerprint, or the cache serves a video of the
/// wrong subject.
#[test]
fn replacing_a_still_changes_the_fingerprint() {
    let (dir, mut project, id) = sketched("stills", "the subject, walking");
    let still = AssetId::new("face");
    project.assets.push(Asset::imported(
        still.clone(),
        AssetKind::Image,
        ProjectPath::new("assets/face.png"),
    ));
    let asset = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == id)
        .expect("the asset");
    asset.video = Some(VideoRequest {
        model: VideoModel::Fast,
        reference_images: vec![still],
        ..VideoRequest::default()
    });

    crate::common::write(&dir, "assets/face.png", "PICTURE ONE");
    let before = fingerprint(&project, &dir, &id);
    crate::common::write(&dir, "assets/face.png", "A DIFFERENT PICTURE");
    let after = fingerprint(&project, &dir, &id);

    assert_ne!(
        before, after,
        "the still's bytes are what is hashed, not its file name"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Where a brief's generation would land.
fn fingerprint(project: &scorsese_core::Project, dir: &std::path::Path, id: &AssetId) -> String {
    let asset = project.asset(id).expect("the asset");
    Brief::of(project, dir, asset)
        .expect("a readable brief")
        .output()
        .as_str()
        .to_owned()
}

/// A ceiling that admits one shot must refuse the second.
///
/// The regression is invisible from the outside without this: every individual
/// number stays correct, and the run reports what it spent only after having
/// spent it. Before the fix, a $1.00 ceiling let twenty 96¢ shots through —
/// each check asked whether *one* more fitted, and one always did.
#[test]
fn a_ceiling_counts_what_this_run_has_already_committed() {
    let (dir, mut project, _) = sketched("running-total", "the first shot");
    project.assets.push(Asset::sketch(
        AssetId::new("second"),
        AssetKind::GeneratedVideo,
        "the second shot",
    ));
    let provider = Mock::answering(
        "shot",
        vec![
            Answer::Ready(b"ONE".to_vec()),
            Answer::Ready(b"TWO".to_vec()),
        ],
    );

    // 150 cents admits one 96-cent shot and cannot admit two.
    let refused = generate(&mut project, &dir, &provider, Budget::new(150, 0))
        .expect_err("two shots at 96 cents is over a 150-cent ceiling");

    assert!(refused.to_string().contains("ceiling"), "{refused}");
    assert_eq!(
        provider.submissions(),
        1,
        "the first shot fits and the second must not"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A ceiling nobody may cross, checked before the call that spends anything.
#[test]
fn a_run_over_the_ceiling_submits_nothing() {
    let (dir, mut project, _) = sketched("budget", "an expensive shot");
    let provider = Mock::answering("shot", vec![Answer::Ready(b"X".to_vec())]);

    let refused = generate(&mut project, &dir, &provider, Budget::new(50, 0))
        .expect_err("96 cents is over a 50-cent ceiling");
    assert!(refused.to_string().contains("ceiling"), "{refused}");
    assert_eq!(provider.submissions(), 0, "the check must come first");
    std::fs::remove_dir_all(&dir).ok();
}
