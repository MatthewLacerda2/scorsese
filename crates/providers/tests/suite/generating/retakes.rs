//! A shot generated again after its brief changed: the old file's
//! measurements must go with it (#1105).
//!
//! Every local caller probes with `Reprobe::Skip`, which skips an asset that
//! already carries `media`, so a block left over from the previous shot would
//! describe it for ever — its length, its raster, whether it has sound.

use scorsese_core::MediaMetadata;
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::generate;

use crate::generating::mock::{Answer, Mock};
use crate::generating::sketched;

#[test]
fn a_shot_generated_again_is_measured_again() {
    let (dir, mut project, id) = sketched("retake", "a red balloon at dusk");
    let provider = Mock::answering(
        "shot",
        vec![
            Answer::Ready(b"FIRST".to_vec()),
            Answer::Ready(b"SECOND".to_vec()),
        ],
    );
    let run = |project: &mut _| {
        generate(project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    };
    run(&mut project);
    run(&mut project);
    let first = project.asset(&id).and_then(|asset| asset.path.clone());
    let asset = project
        .assets
        .iter_mut()
        .find(|asset| asset.id == id)
        .expect("the asset");
    asset.media = Some(MediaMetadata {
        duration_seconds: Some(4.0),
        ..MediaMetadata::default()
    });
    asset.prompt = Some(String::from("a blue balloon at dawn"));

    run(&mut project);
    run(&mut project);

    let asset = project.asset(&id).expect("the asset");
    assert!(first.is_some() && asset.path != first, "a new file");
    assert!(
        asset.media.is_none(),
        "the previous shot's measurements must not survive, or the probe skips it"
    );
    assert_eq!(provider.submissions(), 2);
    std::fs::remove_dir_all(&dir).ok();
}
