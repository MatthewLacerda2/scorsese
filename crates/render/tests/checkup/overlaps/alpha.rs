//! A picture with alpha over another: the same canvas a page is (#967).

use std::path::PathBuf;

use scorsese_core::{Asset, AssetKind, MediaMetadata, Project};

use super::collisions_in;
use super::pages::stacked;
use crate::common::ffmpeg::fixture_dir;
use crate::common::{extension, file_asset};

/// A 1920x1080 source of `kind` whose probe answered `has_alpha` with `alpha`,
/// or that nobody probed for alpha when it is `None`.
fn probed(kind: AssetKind, alpha: Option<bool>) -> Asset {
    Asset {
        media: Some(MediaMetadata {
            width: Some(1920),
            height: Some(1080),
            has_alpha: alpha,
            ..MediaMetadata::default()
        }),
        ..file_asset("layer", kind)
    }
}

/// A project folder holding the source's file, which is all placing it asks
/// of the disk: its size is the one the document records.
fn on_disk(kind: AssetKind) -> PathBuf {
    let dir = fixture_dir("alpha-overlap");
    let file = dir.join(format!("assets/layer.{}", extension(kind)));
    std::fs::create_dir_all(dir.join("assets")).expect("create assets");
    std::fs::write(file, b"").expect("write source");
    dir
}

/// How many collisions a pair of `kind` layers stacked as #807's pages are
/// reported as having, with their probe saying `alpha`.
fn reports(kind: AssetKind, alpha: Option<bool>) -> Vec<String> {
    let project: Project = stacked(probed(kind, alpha));
    collisions_in(&project, &on_disk(kind))
}

#[test]
fn a_video_or_image_with_alpha_says_nothing_about_what_it_hides() {
    // Scenes captured from pages as ffv1/bgra and imported over a background:
    // their rectangle is the frame, what they draw is not.
    assert_eq!(reports(AssetKind::Video, Some(true)), Vec::<String>::new());
    assert_eq!(reports(AssetKind::Image, Some(true)), Vec::<String>::new());
}

#[test]
fn an_opaque_or_unprobed_source_is_still_judged_by_its_rectangle() {
    // The same pair with the alpha taken away is reported — so the silence
    // above is the alpha's doing, not a file nothing could place.
    assert_eq!(reports(AssetKind::Video, Some(false)).len(), 1);
    // Absent is nobody having looked, not a transparent source.
    assert_eq!(reports(AssetKind::Video, None).len(), 1);
}
