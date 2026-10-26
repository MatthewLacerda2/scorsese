//! What an imported file is taken to be, and what happens when the extension
//! and the file disagree.
//!
//! Guessing wrong here is expensive twice over: the mistake surfaces at render
//! time, and by then it is in a cut rather than in one file.

use scorsese_core::AssetKind;

use crate::common::reload;

use super::{CLIP, Outside};

/// Two seconds of a tone, which is what an `audio` import is.
const TONE: &str = "sine=frequency=440:duration=2:sample_rate=48000";

#[test]
fn the_extension_is_what_the_kind_is_inferred_from() {
    let outside = Outside::new("inferred");
    outside.import(&outside.media("bed.wav", TONE), &[]).ok();
    outside.import(&outside.still("card.png"), &[]).ok();

    let kinds: Vec<_> = reload(&outside.project)
        .assets
        .iter()
        .map(|asset| asset.kind)
        .collect();
    assert_eq!(kinds, [AssetKind::Audio, AssetKind::Image]);
    outside.clean();
}

#[test]
fn the_kind_flag_overrides_what_the_extension_says() {
    // A `.mp4` that is in the project for its sound. It really does carry one,
    // so nothing here is a lie — the extension is simply not the answer.
    let outside = Outside::new("override");
    let source = outside.media("interview.mp4", TONE);
    outside.import(&source, &["--kind", "audio"]).ok();

    let project = reload(&outside.project);
    assert_eq!(project.assets[0].kind, AssetKind::Audio);
    outside.clean();
}

#[test]
fn an_extension_nothing_recognises_is_refused_until_the_kind_is_named() {
    // Guessing would import the file as the wrong kind and only surface at
    // render time, so the refusal is the feature.
    let outside = Outside::new("unknown");
    let source = outside.media("bed.wav", TONE);
    let renamed = source.with_extension("dat");
    std::fs::rename(&source, &renamed).expect("rename the fixture");

    let refused = outside.import(&renamed, &[]);
    assert!(refused.failed, "an unknown extension cannot be guessed at");
    refused.says("cannot tell what kind of media");
    assert!(reload(&outside.project).assets.is_empty());
    assert!(outside.pool().is_empty(), "nothing was copied in");

    outside.import(&renamed, &["--kind", "audio"]).ok();
    assert_eq!(reload(&outside.project).assets[0].kind, AssetKind::Audio);
    outside.clean();
}

#[test]
fn an_extension_that_lies_is_refused_before_anything_is_copied() {
    // A file named `.wav` with no sound in it. Caught at import, while it is
    // still one file rather than a broken cut — and the project directory has
    // to be exactly as it was.
    let outside = Outside::new("liar");
    let source = outside.media("silent.mp4", CLIP);
    let renamed = source.with_extension("wav");
    std::fs::rename(&source, &renamed).expect("rename the fixture");

    let refused = outside.import(&renamed, &[]);
    assert!(refused.failed, "a silent `.wav` is not an audio file");
    refused.says("no audio stream");
    assert!(reload(&outside.project).assets.is_empty());
    assert!(outside.pool().is_empty(), "a failed import copies nothing");
    outside.clean();
}
