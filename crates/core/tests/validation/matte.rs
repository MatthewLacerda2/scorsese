//! What a clip's `matte` may name.
//!
//! Five refusals, all answered from the document: a matte that is nowhere, the
//! clip itself, a clip across a group's edge, a clip on an audio track, and a
//! matte with a matte of its own. The last forecloses every chain, so nothing
//! drawing a frame has to walk one.

use crate::common::{assert_only_problem, clip_id, document, project};
use scorsese_core::{Matte, MatteProblem as M, Project};

/// The fixture with `clip` masked by `matte`.
fn masked(clip: &str, matte: &str) -> Project {
    let mut p = project();
    mask(&mut p, clip, matte);
    p
}

fn mask(p: &mut Project, clip: &str, matte: &str) {
    let clip = p
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|c| c.id.as_str() == clip)
        .expect("the fixture has the clip");
    clip.matte = Some(Matte::new(clip_id(matte)));
}

#[test]
fn a_clip_masked_by_another_on_screen_is_valid() {
    assert!(masked("c-logo", "c-title").is_valid());
}

#[test]
fn a_matte_naming_nothing_is_refused() {
    assert_only_problem(
        &masked("c-logo", "c-nope"),
        M::Missing {
            clip: clip_id("c-logo"),
            matte: clip_id("c-nope"),
        },
    );
}

#[test]
fn a_clip_cannot_be_its_own_matte() {
    assert_only_problem(
        &masked("c-logo", "c-logo"),
        M::Itself {
            clip: clip_id("c-logo"),
        },
    );
}

/// Either end on an audio track: a narration clip has no picture to mask with.
#[test]
fn a_matte_on_an_audio_track_is_refused() {
    assert_only_problem(
        &masked("c-logo", "c-vo"),
        M::NotPicture {
            clip: clip_id("c-logo"),
            matte: clip_id("c-vo"),
            on_sound: clip_id("c-vo"),
        },
    );
}

#[test]
fn a_matte_with_a_matte_of_its_own_is_refused() {
    let mut p = masked("c-shot", "c-logo");
    mask(&mut p, "c-logo", "c-title");
    assert_only_problem(
        &p,
        M::Chained {
            clip: clip_id("c-shot"),
            matte: clip_id("c-logo"),
            next: clip_id("c-title"),
        },
    );
}

/// A clip outside a group masked by one of the group's members.
#[test]
fn a_matte_across_a_groups_edge_is_refused() {
    let json = document(
        r##""assets": [
            { "id": "dot", "kind": "color", "color": "#ffffff" },
            { "id": "g", "kind": "group", "group": { "tracks": [
                { "id": "g1", "kind": "video", "clips": [
                    { "id": "c-inside", "asset": "dot", "start": 0, "duration": 10 }] }] } }],
           "tracks": [{ "id": "v1", "kind": "video", "clips": [
               { "id": "c-group", "asset": "g", "start": 0, "duration": 10 }] },
             { "id": "v2", "kind": "video", "clips": [
               { "id": "c-out", "asset": "dot", "start": 0, "duration": 10,
                 "matte": { "clip": "c-inside" } }] }]"##,
    );
    let p = Project::from_json(&json).expect("parses");
    assert_only_problem(
        &p,
        M::Across {
            clip: clip_id("c-out"),
            matte: clip_id("c-inside"),
        },
    );
}
