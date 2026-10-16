//! An imported file's stock `source` (#1237): only on what a library supplies,
//! and only a record somebody could follow.

use crate::common::{assert_only_problem, asset_id, asset_mut, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, Source, SourceProblem, StockLibrary,
};

fn pixabay() -> Source {
    Source {
        library: StockLibrary::Pixabay,
        id: "39009".to_owned(),
        author: Some("someone".to_owned()),
        page_url: Some("https://pixabay.com/videos/id-39009/".to_owned()),
        licence: Some("Pixabay Content License".to_owned()),
    }
}

#[test]
fn an_image_from_a_library_is_valid() {
    let mut p = project();
    asset_mut(&mut p, "logo").source = Some(pixabay());
    p.validate()
        .expect("a stock image with its source is valid");
}

#[test]
fn a_source_on_a_title_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "title").source = Some(pixabay());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("title"),
            field: F::Source,
            kind: AssetKind::Text,
        },
    );
}

#[test]
fn a_source_whose_page_is_not_on_the_web_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "logo").source = Some(Source {
        page_url: Some("javascript:alert(1)".to_owned()),
        ..pixabay()
    });
    assert_only_problem(
        &p,
        E::BadSource {
            asset: asset_id("logo"),
            problem: SourceProblem::PageNotWeb,
        },
    );
}
