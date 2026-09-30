//! A matte never on screen with the clip it masks: said, worded for which way
//! round it is, and quiet the moment the two share an instant.

use scorsese_core::{ClipId, HashCheck, Matte, Project};
use scorsese_render::Checkup;

use crate::common::{clip, project, text_asset, video_track};

/// `shown` over 0–30, masked by `matte` placed at `matte_start` for ten
/// frames.
fn masked(matte_start: u64, invert: bool) -> Project {
    let mut shown = clip("shown", "title", 0, 30);
    shown.matte = Some(Matte {
        clip: ClipId::new("matte"),
        invert,
    });
    project(
        vec![text_asset("title")],
        vec![
            video_track("v1", vec![shown]),
            video_track("v2", vec![clip("matte", "title", matte_start, 10)]),
        ],
    )
}

fn about_mattes(project: &Project) -> Vec<String> {
    let nowhere = std::env::temp_dir().join("scorsese-checkup-no-such-project.scor");
    Checkup::of(project, &nowhere, HashCheck::Skip)
        .lines()
        .iter()
        .filter(|line| line.says.contains("matte"))
        .map(|line| line.says.clone())
        .collect()
}

#[test]
fn a_matte_that_never_meets_its_clip_is_said_to_hide_it() {
    let said = about_mattes(&masked(40, false));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("`shown`") && said[0].contains("never appears"),
        "{}",
        said[0]
    );
}

#[test]
fn inverted_it_is_said_to_cut_nothing() {
    let said = about_mattes(&masked(40, true));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("cuts anything out"), "{}", said[0]);
}

#[test]
fn a_matte_sharing_any_instant_is_not_mentioned() {
    assert_eq!(about_mattes(&masked(25, false)), Vec::<String>::new());
}
