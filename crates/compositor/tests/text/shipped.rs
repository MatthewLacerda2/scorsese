//! The families scorsese ships, and what each refusal says.
//!
//! The catalogue is data, so most of what could go wrong here is a name that
//! resolves to the wrong bytes or a rule that quietly does not fire. Both are
//! invisible in a render — a title in the wrong face still looks like a title —
//! so they are asserted rather than looked at.

use scorsese_compositor::text::{self, Font, FontError};
use scorsese_core::Rgba;

use crate::ink::{self, canvas, style};

fn set_in(font: &Font) -> scorsese_compositor::Frame {
    let mut frame = canvas();
    text::draw(&mut frame, "Chapter One", font, &style(32.0, Rgba::WHITE));
    frame
}

/// Every name the catalogue publishes has to resolve, or the list is a promise
/// nothing keeps — and a name published but unreachable is the exact drift the
/// list sits beside the files to prevent.
#[test]
fn every_published_name_resolves() {
    for name in text::names() {
        let font = Font::shipped(name, None)
            .unwrap_or_else(|error| panic!("`{name}` is published but does not resolve: {error}"));
        assert!(
            ink::count(&set_in(&font)) > 0,
            "`{name}` resolved but drew nothing"
        );
    }
}

/// `sans` and `serif` are aliases, not faces of their own. Every project ever
/// written names one of them, so the two doors have to lead to the same room.
#[test]
fn the_aliases_reach_the_faces_they_stand_for() {
    let by_alias = Font::shipped("sans", None).expect("sans resolves");
    let by_name = Font::shipped("inter", None).expect("inter resolves");
    assert_eq!(
        set_in(&by_alias).bytes(),
        set_in(&by_name).bytes(),
        "`sans` and `inter` have to be the same face"
    );
    assert_eq!(
        set_in(&Font::shipped("serif", None).expect("serif resolves")).bytes(),
        set_in(&Font::shipped("source-serif", None).expect("source-serif resolves")).bytes(),
    );
    // And the cheap statics agree with the door that takes a weight, or the
    // common case would render differently from the weighted one.
    assert_eq!(set_in(Font::sans()).bytes(), set_in(&by_name).bytes());
}

/// The whole reason Liberation could not be shipped once `weight` existed: it
/// has no axis, and its weights are two separate files.
#[test]
fn a_drawn_family_sets_the_weights_it_was_drawn_at() {
    let regular = Font::shipped("liberation-sans", Some(400)).expect("Regular");
    let bold = Font::shipped("liberation-sans", Some(700)).expect("Bold");

    assert!(
        ink::count(&set_in(&bold)) > ink::count(&set_in(&regular)),
        "Bold has to put down more ink than Regular; {} against {}",
        ink::count(&set_in(&bold)),
        ink::count(&set_in(&regular))
    );
}

/// Refused, not snapped to 700. Snapping is the same silent substitution the
/// variable rules already refuse at the end of an axis, and the message has to
/// say what to write instead.
#[test]
fn a_drawn_family_refuses_a_weight_nobody_drew() {
    let error = Font::shipped("liberation-sans", Some(600)).expect_err("600 was never drawn");

    assert!(
        matches!(&error, FontError::WeightNotDrawn { weight, .. } if *weight == 600),
        "got `{error}`"
    );
    let message = error.to_string();
    assert!(
        message.contains("400") && message.contains("700"),
        "the refusal has to name the weights there are; got `{message}`"
    );
}

/// A variable family keeps answering with its range, which is a different
/// sentence from a drawn one's list and has to stay one.
#[test]
fn a_variable_family_refuses_a_weight_off_its_axis() {
    // Lora stops at 700, where Inter reaches 900 — so this is a fact about the
    // family rather than about the number.
    let error = Font::shipped("lora", Some(900)).expect_err("Lora stops at 700");

    assert!(
        matches!(error, FontError::WeightOffAxis { weight, .. } if weight == 900),
        "got `{error}`"
    );
    assert!(
        Font::shipped("inter", Some(900)).is_ok(),
        "900 is not the problem; Lora is"
    );
}

/// The question somebody is actually asking when they get a name wrong.
#[test]
fn an_unknown_name_is_refused_with_the_list() {
    let error = Font::shipped("Arial", None).expect_err("Arial cannot be shipped");

    let message = error.to_string();
    assert!(
        matches!(error, FontError::NoSuchFamily { .. }),
        "got `{message}`"
    );
    for expected in ["liberation-sans", "inter", "serif"] {
        assert!(
            message.contains(expected),
            "the refusal has to carry the list, and `{expected}` was not in `{message}`"
        );
    }
}
