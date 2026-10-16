//! A change of shape (#1278): footage that filled the old frame fills the
//! new one, and what only a layout can see is said in the checkup's words.

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fit, MediaMetadata, Ratio, TextStyle, frame::Refit,
};
use scorsese_render::reframe;

use crate::common::{file_asset, project, video_track};

fn nowhere() -> std::path::PathBuf {
    std::env::temp_dir().join("scorsese-reframe-no-such-project.scor")
}

/// A landscape shot under a headline that fits one landscape line.
fn landscape() -> scorsese_core::Project {
    let shot = Asset {
        media: Some(MediaMetadata {
            width: Some(1920),
            height: Some(1080),
            ..MediaMetadata::default()
        }),
        ..file_asset("field", AssetKind::Video)
    };
    let title = Asset {
        style: Some(TextStyle {
            size: 0.2,
            ..TextStyle::default()
        }),
        ..Asset::text(AssetId::new("headline"), "A HEADLINE ACROSS THE FIELD")
    };
    project(
        vec![shot, title],
        vec![
            video_track("v1", vec![Clip::at("shot", "field", 0, 60)]),
            video_track("v2", vec![Clip::at("words", "headline", 0, 60)]),
        ],
    )
}

#[test]
fn upright_the_shot_fills_and_the_headline_is_said_to_be_cut() {
    let mut project = landscape();
    assert!(
        reframe(&mut project, &nowhere(), Ratio::WIDE).unsettled.is_empty(),
        "the same shape reads nothing"
    );
    let reframed = reframe(&mut project, &nowhere(), Ratio::TALL);
    assert_eq!(
        reframed.refit,
        Refit {
            from: Ratio::WIDE,
            to: Ratio::TALL,
            filled: vec![ClipId::new("shot")],
        }
    );
    assert_eq!(project.aspect, Ratio::TALL);
    assert_eq!(project.tracks[0].clips[0].fit, Fit::Fill);
    let said = reframed.to_string();
    assert!(said.starts_with("Reframed from 16:9 to 9:16."), "{said}");
    assert!(said.contains("1 clip that filled the old frame fills the new one"), "{said}");
    assert!(said.contains("text clip `words` is cut off"), "{said}");
}
