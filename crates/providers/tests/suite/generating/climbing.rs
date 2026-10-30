//! A shot named for an id that climbs out of the project is refused before it
//! is submitted: refused while refusing still costs nothing.

use scorsese_core::AssetId;
use scorsese_providers::credentials::Budget;
use scorsese_providers::video::generate;

use crate::generating::mock::Mock;
use crate::generating::sketched;

#[test]
fn a_shot_whose_id_climbs_is_never_submitted() {
    let (dir, mut project, _) = sketched("climbs", "a lighthouse at dusk");
    project.assets[0].id = AssetId::new("../../escaped");
    let provider = Mock::answering("../../escaped", Vec::new());

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0));

    assert_eq!(provider.submissions(), 0, "nothing was paid for: {run:?}");
    std::fs::remove_dir_all(&dir).ok();
}
