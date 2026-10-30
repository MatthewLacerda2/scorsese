//! A still named for an id that climbs out of the project is refused before it
//! is drawn: refused while refusing still costs nothing.

use scorsese_core::AssetId;
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::generate;

use crate::drawing::mock::Mock;
use crate::drawing::sketched;

#[test]
fn a_still_whose_id_climbs_is_never_drawn() {
    let (dir, mut project, _) = sketched("climbs", "a lighthouse at dusk");
    project.assets[0].id = AssetId::new("../../escaped");
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0));

    assert_eq!(provider.requests(), 0, "nothing was paid for: {run:?}");
    std::fs::remove_dir_all(&dir).ok();
}
