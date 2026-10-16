//! A bake's body read against a record: which sections count as the body, and
//! what each row calls out.

use scorsese_render::audio::reference::find;
use scorsese_render::say::likeness;
use scorsese_zimmer::level::{Bands, Loudness, Profile, Span};

/// A section from `from` to `to` seconds at this mean and crest, these band
/// shares in percent, and this width.
fn span(from: f64, to: f64, mean: f64, crest: f64, bands: [f64; 3], corr: f64) -> Span {
    Span {
        label: None,
        from_seconds: from,
        to_seconds: to,
        loudness: Loudness {
            peak_dbfs: Some(mean + crest),
            true_peak_dbfs: Some(mean + crest),
            mean_dbfs: Some(mean),
        },
        bands: Some(Bands {
            low: bands[0] / 100.0,
            mid: bands[1] / 100.0,
            high: bands[2] / 100.0,
        }),
        correlation: Some(corr),
    }
}

/// Sections of eight seconds each, and a whole that spans them.
fn profile(sections: Vec<Span>) -> Profile {
    let end = sections.last().map_or(8.0, |it| it.to_seconds);
    Profile {
        whole: span(0.0, end, -16.0, 16.0, [70.0, 26.0, 4.0], 0.8),
        sections,
    }
}

/// A body that sits where Billie Jean's does.
fn groove(from: f64) -> Span {
    span(from, from + 8.0, -16.5, 15.5, [75.0, 22.0, 3.0], 0.8)
}

fn row<'a>(lines: &'a [String], name: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.starts_with(name))
        .unwrap_or_else(|| panic!("no `{name}` row in {lines:#?}"))
}

#[test]
fn a_body_like_the_record_s_is_inside_on_every_field() {
    let sections = vec![groove(0.0), groove(8.0), groove(16.0), groove(24.0)];
    let lines = likeness(&profile(sections), find("billie-jean").expect("measured"));
    assert!(lines[0].contains("Billie Jean") && lines[0].contains("8.000-24.000"));
    assert!(
        lines[0].contains("(2 sections)"),
        "the first and last are not the body"
    );
    for name in ["crest", "low %", "mid %", "high %", "corr", "opening"] {
        assert!(row(&lines, name).ends_with("inside"), "{name}: {lines:#?}");
    }
    assert!(
        row(&lines, "mean").contains("near -16.5"),
        "the reachable mean"
    );
}

/// The anti-reference, as `docs/references.md` records it: a quiet, hat-heavy
/// opening and a body lighter and boxier than the record.
#[test]
fn the_anti_reference_s_failures_are_each_called_out() {
    let opening = span(0.0, 8.0, -25.3, 19.0, [40.0, 38.0, 22.0], 0.8);
    let light = |from| span(from, from + 8.0, -20.0, 17.0, [52.0, 40.0, 8.0], 0.4);
    let sections = vec![opening, light(8.0), light(16.0), groove(24.0), light(32.0)];
    let lines = likeness(&profile(sections), find("billie-jean").expect("measured"));
    assert!(
        row(&lines, "low %").contains("2 of 3 lighter"),
        "{lines:#?}"
    );
    assert!(
        row(&lines, "mid %").ends_with("inside"),
        "40% is the range's top"
    );
    assert!(row(&lines, "high %").contains("2 of 3 brighter"));
    assert!(row(&lines, "corr").contains("2 of 3 wider"));
    assert!(row(&lines, "opening").ends_with("quieter than the record opens"));
}

/// Under the record's high share is no finding, because the records' files
/// read low up there. Only over is.
#[test]
fn a_dull_top_is_not_called_out_and_a_flat_crest_is() {
    let dull = |from| span(from, from + 8.0, -16.0, 11.0, [80.0, 20.0, 0.0], 0.8);
    let lines = likeness(
        &profile(vec![dull(0.0), dull(8.0)]),
        find("billie-jean").expect("measured"),
    );
    assert!(row(&lines, "high %").ends_with("inside"));
    assert!(row(&lines, "crest").contains("2 of 2 flatter"));
}

/// A file never split is its own body, and has no opening apart from it.
#[test]
fn an_unsplit_file_is_all_body_and_has_no_opening_row() {
    let lines = likeness(
        &profile(Vec::new()),
        find("techno-syndrome").expect("measured"),
    );
    assert!(lines[0].contains("(1 section)"));
    assert!(
        row(&lines, "corr").contains("mono"),
        "Techno Syndrome's file is mono"
    );
    assert!(!lines.iter().any(|line| line.starts_with("opening")));
}

/// Fuego is a build: opening at full level is the mistake against it.
#[test]
fn an_opening_with_no_build_is_louder_than_a_build_opens() {
    let drop = |from| span(from, from + 8.0, -11.0, 10.0, [80.0, 19.0, 1.0], 0.85);
    let sections = vec![drop(0.0), drop(8.0), drop(16.0)];
    let lines = likeness(&profile(sections), find("fuego-drops").expect("measured"));
    assert!(row(&lines, "opening").ends_with("louder than the record opens"));
}
