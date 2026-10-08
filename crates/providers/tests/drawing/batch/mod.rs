//! Half price within a day (#894): ordered, waited on, collected — and never
//! ordered or drawn twice.

mod collecting;
mod ordering;

use scorsese_core::AssetId;
use scorsese_providers::image::Brief;
use scorsese_providers::prices;

/// What the still `id` costs in a batch, by the one function that prices it.
pub(crate) fn half(project: &scorsese_core::Project, dir: &std::path::Path, id: &AssetId) -> u64 {
    let brief = Brief::of(project, dir, project.asset(id).expect("there")).expect("whole");
    prices::image_in_batch(&brief.request, brief.characters(), 0)
        .expect("priced")
        .cents
}
