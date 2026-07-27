//! What a bake records about itself.
//!
//! Nothing here needs ffprobe: the thing that wrote the file knows what is in
//! it, and an asset that cannot say how long it is makes an agent guess the
//! length of the clip it just made.

use crate::common::{new_project, reload, run_in};

/// Starts a project with one song recipe already baked.
fn baked(label: &str) -> std::path::PathBuf {
    let dir = new_project(label);
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "bake"]).ok();
    dir
}

/// A baked asset has to say how long it is, or an agent laying a clip of the
/// music it just made has to guess. Nothing needs ffprobe to discover this —
/// the bake knows what it wrote.
#[test]
fn a_bake_records_how_long_it_is() {
    let dir = baked("synth-metadata");
    let project = reload(&dir);
    let media = project
        .assets
        .first()
        .expect("one asset")
        .media
        .expect("the bake recorded what it made");

    // The starter song is four 4-beat patterns at 96 bpm — ten seconds — plus
    // the last note's release and the reverb tail.
    let seconds = media.duration_seconds.expect("a duration");
    assert!(
        (10.0..12.0).contains(&seconds),
        "the starter song should be about ten seconds, got {seconds}"
    );
    assert_eq!(media.audio_channels, Some(1));
    assert_eq!(media.sample_rate, Some(44_100));
    assert_eq!(media.width, None, "sound has no picture");
}

/// A cache hit is still a recording: an asset pointed at an existing bake must
/// come away knowing as much as one that rendered it.
#[test]
fn a_cached_bake_records_the_same_thing() {
    let dir = baked("synth-metadata-cached");
    let fresh = reload(&dir).assets.first().expect("one asset").media;
    run_in(&dir, &["synth", "bake"]).ok();
    assert_eq!(reload(&dir).assets.first().expect("one asset").media, fresh);
}
