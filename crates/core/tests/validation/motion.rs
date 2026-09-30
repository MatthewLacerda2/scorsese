//! A text asset's `reveal` and `number` blocks: what the document alone can
//! say will never draw as meant.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{Counter, Reveal, TextProblem as T, TextStyle};

fn styled(reveal: Option<Reveal>, number: Option<Counter>) -> TextStyle {
    TextStyle {
        reveal,
        number,
        ..TextStyle::default()
    }
}

#[test]
fn a_reveal_with_its_defaults_and_a_counter_with_a_placeholder_are_valid() {
    let mut p = project();
    let title = asset_mut(&mut p, "title");
    title.text = Some("{n} partitions".to_owned());
    title.style = Some(styled(Some(Reveal::default()), Some(Counter::default())));
    assert!(problems(&p).is_empty(), "{:?}", problems(&p));
}

/// Both ends of the stagger are meaningful — `0` is a plain fade and `1` a
/// typewriter — so the refusal starts just past each of them.
#[test]
fn a_stagger_outside_nought_to_one_is_refused_and_its_ends_are_not() {
    for stagger in [0.0, 1.0] {
        let mut p = project();
        asset_mut(&mut p, "title").style = Some(styled(
            Some(Reveal {
                stagger,
                ..Reveal::default()
            }),
            None,
        ));
        assert!(problems(&p).is_empty(), "stagger {stagger} is in range");
    }
    for stagger in [-0.1, 1.5, f64::NAN] {
        let mut p = project();
        asset_mut(&mut p, "title").style = Some(styled(
            Some(Reveal {
                stagger,
                ..Reveal::default()
            }),
            None,
        ));
        assert_eq!(problems(&p).len(), 1, "stagger {stagger} is refused");
    }
}

#[test]
fn a_rise_that_is_not_a_distance_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(styled(
        Some(Reveal {
            rise: f64::INFINITY,
            ..Reveal::default()
        }),
        None,
    ));
    assert_only_problem(
        &p,
        T::RiseNotFinite {
            asset: asset_id("title"),
            rise: f64::INFINITY,
        },
    );
}

#[test]
fn a_counter_with_nowhere_to_write_its_figure_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "title").style = Some(styled(None, Some(Counter::default())));
    assert_only_problem(
        &p,
        T::NoPlaceholder {
            asset: asset_id("title"),
        },
    );
}

#[test]
fn a_counter_s_numbers_have_to_be_writable() {
    let mut p = project();
    let title = asset_mut(&mut p, "title");
    title.text = Some("{n}".to_owned());
    title.style = Some(styled(
        None,
        Some(Counter {
            decimals: 7,
            ..Counter::default()
        }),
    ));
    assert_only_problem(
        &p,
        T::TooManyDecimals {
            asset: asset_id("title"),
            decimals: 7,
            max: 6,
        },
    );
    asset_mut(&mut p, "title").style = Some(styled(
        None,
        Some(Counter {
            value: f64::INFINITY,
            ..Counter::default()
        }),
    ));
    assert_only_problem(
        &p,
        T::ValueNotFinite {
            asset: asset_id("title"),
            value: f64::INFINITY,
        },
    );
}
