//! The two things about a chroma key a document can be wrong about: what it is
//! aimed at, and what it is applied to.

use crate::common::{asset_id, assert_only_problem, clip_id, problems, project};
use scorsese_core::{AssetKind, ChromaKey, MIN_KEY_SPREAD, Rgba, TimelineProblem as E};

/// The green most screens are painted, and a grey that is not a screen at all.
const SCREEN: Rgba = Rgba::opaque(0, 177, 64);

/// `c-logo` shows an `image`, which is footage as far as a key is concerned.
#[test]
fn a_key_on_footage_is_fine() {
    let mut p = project();
    p.tracks[1].clips[0].chroma_key = Some(ChromaKey::new(SCREEN));
    assert_eq!(p.validate(), Ok(()));
}

/// `c-title` shows a `text` asset, which this build drew with exactly the alpha
/// the document asked for — so keying it asks the build to undo its own
/// drawing.
#[test]
fn a_key_on_a_kind_that_draws_its_own_alpha_is_refused() {
    let mut p = project();
    p.tracks[0].clips[1].chroma_key = Some(ChromaKey::new(SCREEN));
    assert_only_problem(
        &p,
        E::KeyedInlineAsset {
            clip: clip_id("c-title"),
            asset: asset_id("title"),
            asset_kind: AssetKind::Text,
        },
    );
}

/// And the same for a `color`, which is the other inline kind the fixture
/// carries. One flat colour keys either entirely or not at all, which is
/// `opacity` said a harder way.
#[test]
fn a_key_on_a_colour_card_is_refused_too() {
    let mut p = project();
    p.tracks[0].clips[2].chroma_key = Some(ChromaKey::new(SCREEN));
    assert_only_problem(
        &p,
        E::KeyedInlineAsset {
            clip: clip_id("c-black"),
            asset: asset_id("black"),
            asset_kind: AssetKind::Color,
        },
    );
}

/// A key on a grey has only brightness left to separate pixels by, which is a
/// luma key — and a luma key takes the eyes, the teeth and the shine on the
/// hair with the background.
#[test]
fn a_screen_colour_with_no_hue_is_refused() {
    for grey in [
        Rgba::opaque(255, 255, 255),
        Rgba::opaque(0, 0, 0),
        Rgba::opaque(128, 133, 128),
    ] {
        let mut p = project();
        p.tracks[1].clips[0].chroma_key = Some(ChromaKey::new(grey));
        assert_only_problem(
            &p,
            E::NeutralKeyColor {
                clip: clip_id("c-logo"),
                color: grey,
            },
        );
    }
}

/// The line itself, from both sides: a spread of exactly [`MIN_KEY_SPREAD`] is
/// a colour and one level under it is not.
///
/// Stated as the boundary rather than as two comfortable numbers, because a
/// comparison mutated from `>=` to `>` satisfies every colour but this one.
#[test]
fn the_line_between_a_colour_and_a_grey_is_the_spread_itself() {
    let spread = |by: u8| Rgba::opaque(100, 100 + by, 100);
    assert!(ChromaKey::new(spread(MIN_KEY_SPREAD)).is_keyable());
    assert!(!ChromaKey::new(spread(MIN_KEY_SPREAD - 1)).is_keyable());
    // Whichever channel carries it: the spread is between the strongest and the
    // weakest, not between two named ones.
    let low = Rgba::opaque(100, 100, 100 - MIN_KEY_SPREAD);
    assert!(ChromaKey::new(low).is_keyable());
}

/// Both problems at once, reported together — an agent repairing a project
/// unattended should get the whole list rather than one error per round trip.
#[test]
fn a_grey_key_on_a_title_reports_both() {
    let mut p = project();
    p.tracks[0].clips[1].chroma_key = Some(ChromaKey::new(Rgba::opaque(9, 9, 9)));
    assert_eq!(problems(&p).len(), 2, "{:?}", problems(&p));
}

/// And a clip with no key is not checked for either, which is the common case
/// and the one a check reaching for `Option::unwrap` would break.
#[test]
fn a_clip_with_no_key_is_asked_nothing() {
    assert_eq!(project().validate(), Ok(()));
}
