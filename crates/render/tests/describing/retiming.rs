//! Saying that a clip runs at a rate of its own.

use scorsese_core::{AssetKind, Speed};

use crate::common::{clip, file_asset, project, sounding_asset, sped, video_track};
use crate::described;

/// A speed is invisible in a frame count — a ten-second slot holding twenty
/// seconds of footage looks exactly like a ten-second slot — so a description
/// has to say it out loud, on the picture line and on the sound line both.
#[test]
fn a_clip_running_at_its_own_rate_says_so_on_both_lines() {
    let project = project(
        vec![sounding_asset("shot", AssetKind::Video)],
        vec![video_track(
            "v1",
            vec![sped(2.0, clip("c1", "shot", 0, 60))],
        )],
    );
    let description = described(&project);

    assert_eq!(description.stretches[0].picture[0].speed, Speed::new(2.0));
    assert_eq!(description.stretches[0].sound[0].speed, Speed::new(2.0));
    let said = format!("{description}");
    assert!(
        said.contains("shot (video, fit, 2×)"),
        "the picture line has to name the rate:\n{said}"
    );
    assert!(
        said.contains("shot (video, 2×)"),
        "and so does the sound line:\n{said}"
    );
}

/// And the ordinary clip says nothing at all: `1×` on every line of every
/// description would bury the one shot where the answer is worth reading.
#[test]
fn a_clip_at_its_sources_own_rate_says_nothing_about_speed() {
    let plain = project(
        vec![file_asset("shot", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "shot", 0, 60)])],
    );
    let said = format!("{}", described(&plain));
    assert!(!said.contains('×'), "got:\n{said}");
}
