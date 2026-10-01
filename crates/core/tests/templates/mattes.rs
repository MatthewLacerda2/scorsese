//! A matte travels with the clip it masks: refused when left behind, and
//! renamed along with it when copied.

use scorsese_core::template::{ExtractError, extract, insert};
use scorsese_core::{ClipId, Frames, Matte, Project};

use super::groups::explainer;
use super::{episode, ids};

/// The episode with its hero shot revealed through the box beside it.
fn revealed() -> Project {
    let mut project = episode();
    let hero = project
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|clip| clip.id.as_str() == "c-hero")
        .expect("the fixture has it");
    hero.matte = Some(Matte::new(ClipId::new("c-box")));
    project
}

#[test]
fn a_masked_clip_cannot_leave_its_matte_behind() {
    let error = extract(&revealed(), &ids(&["c-hero"]), "Hero").expect_err("c-box is not chosen");
    assert!(
        matches!(error, ExtractError::MatteUnchosen { .. }),
        "got {error}"
    );
}

#[test]
fn a_copied_clip_is_masked_by_the_copied_matte() {
    let reveal = extract(&revealed(), &ids(&["c-hero", "c-box"]), "Reveal").expect("stands");
    let mut project = revealed();
    insert(&mut project, &reveal, Frames(900)).expect("it lands");
    let copy = project
        .clips()
        .find(|(_, clip)| clip.id.as_str() == "c-hero-2")
        .expect("copied")
        .1;
    assert_eq!(copy.matte, Some(Matte::new(ClipId::new("c-box-2"))));
}

/// The explainer with its diagram's arrow member seen through `matte`.
fn matted_member(matte: &str) -> Project {
    let mut project = explainer();
    let diagram = project
        .assets
        .iter_mut()
        .find_map(|asset| {
            asset
                .group
                .as_mut()
                .filter(|_| asset.id.as_str() == "diagram")
        })
        .expect("the fixture has it");
    let arrow = diagram
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|clip| clip.id.as_str() == "m-arrow")
        .expect("the fixture has it");
    arrow.matte = Some(Matte::new(ClipId::new(matte)));
    project
}

/// A member masked by another member of its group: the copy's member is
/// masked by the copy's, never by the original's.
#[test]
fn a_copied_member_is_masked_by_the_copied_member() {
    let project = matted_member("m-box");
    project
        .validate()
        .expect("a matte inside its group is valid");
    let saved = extract(&project, &ids(&["c-diagram"]), "Diagram").expect("all carried");
    let mut project = project;
    insert(&mut project, &saved, Frames(600)).expect("it lands");
    let copy = project
        .every_clip()
        .find(|(_, clip)| clip.id.as_str() == "m-arrow-2")
        .expect("copied")
        .1;
    assert_eq!(copy.matte, Some(Matte::new(ClipId::new("m-box-2"))));
}

/// A member masked by a clip the template leaves behind is refused by name,
/// as a chosen clip is — validation already refuses a matte across a group's
/// edge, so this document is one only a hand-built project can be.
#[test]
fn a_member_cannot_leave_its_matte_behind() {
    let error = extract(&matted_member("bg"), &ids(&["c-diagram"]), "Diagram")
        .expect_err("bg is not chosen");
    assert!(
        matches!(&error, ExtractError::MatteUnchosen { clip, matte }
            if clip.as_str() == "m-arrow" && matte.as_str() == "bg"),
        "got {error}"
    );
}
