//! A matte travels with the clip it masks: refused when left behind, and
//! renamed along with it when copied.

use scorsese_core::template::{ExtractError, extract, insert};
use scorsese_core::{ClipId, Frames, Matte, Project};

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
