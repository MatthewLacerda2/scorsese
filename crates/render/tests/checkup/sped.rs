//! A sped-up clip that plays past the end of its footage (#1098).
//!
//! It loaded before speed counted against the source, so it still loads, and
//! the checkup is where it is said: the render goes on decoding after the end.

use scorsese_core::{AssetKind, HashCheck, MediaMetadata, Project};
use scorsese_render::Checkup;
use scorsese_render::checkup::Severity;

use crate::common::{clip, file_asset, project, sped, video_track};

/// A ten-second shot (300 frames at 30fps) under a 200-frame clip at `rate`.
fn shot_at(rate: f64) -> Project {
    let mut shot = file_asset("shot", AssetKind::Video);
    shot.media = Some(MediaMetadata {
        duration_seconds: Some(10.0),
        ..MediaMetadata::default()
    });
    project(
        vec![shot],
        vec![video_track(
            "v1",
            vec![sped(rate, clip("c-shot", "shot", 0, 200))],
        )],
    )
}

fn checkup(project: &Project) -> Checkup {
    let dir = std::env::temp_dir().join("scorsese-checkup-sped-nowhere.scor");
    Checkup::of(project, &dir, HashCheck::Skip)
}

fn warnings(checkup: &Checkup) -> Vec<String> {
    checkup
        .lines()
        .iter()
        .filter(|line| line.severity == Severity::Warning && line.says.contains("c-shot"))
        .map(|line| line.says.clone())
        .collect()
}

#[test]
fn a_sped_up_clip_past_its_footage_is_a_warning() {
    let checkup = checkup(&shot_at(2.0));
    assert_eq!(
        warnings(&checkup),
        [
            "clip `c-shot` is sped up to reach 400f into asset `shot`, which is 300f long, so \
          its end plays past the footage; shorten it or slow it down"
        ]
    );
    assert!(
        !checkup.verdict().says().contains("c-shot"),
        "a warning, never a problem"
    );
}

#[test]
fn a_sped_up_clip_inside_its_footage_says_nothing() {
    assert!(warnings(&checkup(&shot_at(1.5))).is_empty());
}
