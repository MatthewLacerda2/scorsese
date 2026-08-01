//! How a rate is written down, and which ones are refused.

use scorsese_core::{Project, Speed, TimelineProblem, ValidationError};

use crate::common;

/// A document with one clip carrying `body` among its fields.
fn with_clip(body: &str) -> String {
    common::document(&format!(
        r#""assets": [{{ "id": "a1", "kind": "video", "path": "assets/a.mp4" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c1", "asset": "a1", "start": 0, "duration": 30{body} }}
           ]}}]"#
    ))
}

fn speed_of(json: &str) -> Speed {
    let project = Project::from_json(json).expect("the document parses");
    project.clips().next().expect("a clip").1.speed
}

/// The reason the field is optional: every project authored before it existed
/// goes on playing at exactly the rate it always did.
#[test]
fn a_clip_that_says_nothing_plays_at_its_sources_own_rate() {
    assert_eq!(speed_of(&with_clip("")), Speed::NORMAL);
    assert_eq!(Speed::default(), Speed::NORMAL);
    assert_eq!(Speed::NORMAL.get(), 1.0);
}

#[test]
fn a_rate_is_read_as_written() {
    assert_eq!(speed_of(&with_clip(r#", "speed": 2.0"#)).get(), 2.0);
    assert_eq!(speed_of(&with_clip(r#", "speed": 0.25"#)).get(), 0.25);
}

#[test]
fn the_absent_case_is_not_written_back_out() {
    let project = Project::from_json(&with_clip("")).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(
        !json.contains("\"speed\""),
        "a clip at the ordinary rate must not grow a field it did not have:\n{json}"
    );
}

/// A zero or a negative rate parses perfectly well — unlike a fractional frame
/// count, which cannot be a `u64` — so it reaches validation, which is where it
/// can be reported with the clip's name rather than with a line number.
#[test]
fn a_rate_nothing_could_play_at_is_refused_by_name() {
    for rate in ["0.0", "-1.5"] {
        let project = Project::from_json(&with_clip(&format!(r#", "speed": {rate}"#)))
            .expect("a float parses whatever sign it has");
        let errors = project.validate().expect_err("it does not validate");
        assert!(
            errors.as_slice().iter().any(
                |error| matches!(error, ValidationError::Timeline(TimelineProblem::UnusableSpeed { clip, .. })
                    if clip.as_str() == "c1")
            ),
            "speed {rate} should be refused, and gave {errors:?}"
        );
    }
}

/// JSON has no way to write an infinity or a NaN, so the only way one exists is
/// a project built in memory — and validation catches it there, which is what
/// stops a nonsense rate from reaching a decoder as a filter argument.
#[test]
fn a_rate_that_is_not_a_number_is_refused_too() {
    for rate in [f64::INFINITY, f64::NAN] {
        let mut project = Project::from_json(&with_clip("")).expect("parses");
        project.tracks[0].clips[0].speed = Speed::new(rate);
        let errors = project.validate().expect_err("it does not validate");
        assert!(
            errors.as_slice().iter().any(|error| matches!(
                error,
                ValidationError::Timeline(TimelineProblem::UnusableSpeed { .. })
            )),
            "speed {rate} should be refused, and gave {errors:?}"
        );
    }
}

#[test]
fn an_ordinary_rate_validates() {
    for rate in ["0.1", "1.0", "4.0"] {
        let project =
            Project::from_json(&with_clip(&format!(r#", "speed": {rate}"#))).expect("parses");
        assert_eq!(project.validate(), Ok(()), "speed {rate} is a rate");
    }
}
