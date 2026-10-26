//! What was heard is written only if the recording is still the one heard:
//! a project changed while the server listened writes nothing.

use scorsese_core::AssetId;
use scorsese_providers::transcribe::{TranscribeError, hear, keep};

use super::run::{Ear, asked, folder, hearing};

#[test]
fn a_recording_removed_or_replaced_while_heard_is_not_written_for() {
    let (dir, mut project) = folder("kept");
    let take = AssetId::new("take");
    project.assets[0].sha256 = Some("first".into());
    let ear = Ear::default();
    let done = hear(&project, &dir, &take, hearing(&asked(false), &ear)).expect("heard");
    let words = dir.join("assets/take.mp4.words.json");

    let mut replaced = project.clone();
    replaced.assets[0].sha256 = Some("second".into());
    let mut removed = project.clone();
    removed.assets.retain(|asset| asset.id != take);
    let mut moved = project.clone();
    moved.assets[0].path = Some(scorsese_core::ProjectPath::new("assets/other.mp4"));
    for now in [&replaced, &removed, &moved] {
        let refused = keep(now, &dir, &done).expect_err("changed while heard");
        assert!(
            matches!(&refused, TranscribeError::Changed(asset) if *asset == take),
            "{refused:?}"
        );
        assert!(
            refused.to_string().contains("transcribe it again"),
            "{refused}"
        );
        assert!(!words.exists(), "nothing is written");
    }

    keep(&project, &dir, &done).expect("still the recording heard");
    assert!(words.exists());
    std::fs::remove_dir_all(&dir).ok();
}
