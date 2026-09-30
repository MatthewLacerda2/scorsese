//! What a document says about a clip's light of its own — `shadow`, `glow` and
//! `blend` — and what it leaves out.
//!
//! The three share the round-trip rule every look on a clip keeps: neutral
//! means absent, absent writes nothing, so a project that says nothing about
//! them comes back out of a load-and-save exactly as it went in.

mod common;

use scorsese_core::{Blend, Glow, Project, Rgba, Shadow};

/// A document with one image clip, carrying `looks` as its extra fields.
fn clip_with(looks: &str) -> Project {
    let json = common::document(&format!(
        r#""assets": [{{ "id": "a", "kind": "image", "path": "assets/a.png" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c", "asset": "a", "start": 0, "duration": 30 {looks} }}] }}]"#
    ));
    Project::from_json(&json).expect("parses")
}

fn only_clip(project: &Project) -> &scorsese_core::Clip {
    project.clips().next().expect("a clip").1
}

#[test]
fn a_clip_that_says_nothing_has_no_light_and_writes_none() {
    let project = clip_with("");
    let clip = only_clip(&project);
    assert_eq!(
        (clip.shadow, clip.glow, clip.blend),
        (None, None, Blend::Normal)
    );
    let json = project.to_json().expect("serialise");
    for key in ["shadow", "glow", "blend"] {
        assert!(!json.contains(key), "no `{key}` invented: {json}");
    }
}

/// `{}` is the effect at its defaults, so asking for one without knowing its
/// numbers still does something — and a glow's colour defaults to none, which
/// is the layer's own.
#[test]
fn an_empty_shadow_or_glow_is_one_at_the_defaults() {
    let project = clip_with(r#", "shadow": {}, "glow": {}"#);
    let clip = only_clip(&project);
    assert_eq!(clip.shadow, Some(Shadow::default()));
    assert_eq!(clip.glow, Some(Glow::default()));
    let glow = Glow::default();
    assert_eq!(glow.color, None, "the layer's own colours");
    assert!(glow.radius > 0.0 && glow.intensity > 0.0, "and visible");
    let shadow = Shadow::default();
    assert_eq!(shadow.color, Rgba::BLACK);
    assert!(shadow.opacity > 0.0 && shadow.offset_x > 0.0 && shadow.offset_y > 0.0);
}

#[test]
fn every_number_somebody_chose_survives_a_save() {
    let project = clip_with(
        r##", "blend": "screen",
             "shadow": { "color": "#102030ff", "offset_x": -0.02, "offset_y": 0.03,
                         "softness": 0.04, "opacity": 0.7 },
             "glow": { "color": "#ffcc00ff", "radius": 0.05, "intensity": 2.5 }"##,
    );
    let clip = only_clip(&project);
    assert_eq!(clip.blend, Blend::Screen);
    assert_eq!(
        clip.shadow,
        Some(Shadow {
            color: Rgba::opaque(0x10, 0x20, 0x30),
            offset_x: -0.02,
            offset_y: 0.03,
            softness: 0.04,
            opacity: 0.7,
        })
    );
    assert_eq!(
        clip.glow.and_then(|glow| glow.color),
        Some(Rgba::opaque(255, 204, 0))
    );
    let json = project.to_json().expect("serialise");
    assert!(json.contains(r#""blend": "screen""#), "{json}");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
    assert_eq!(project.validate(), Ok(()));
}

/// The list is four and closed; a fifth mode is refused rather than read as
/// `normal`, and so is a key a shadow does not have.
#[test]
fn a_blend_or_a_key_it_does_not_know_is_refused() {
    for looks in [
        r#", "blend": "overlay""#,
        r#", "shadow": { "angle": 45 }"#,
        r#", "glow": { "spread": 1 }"#,
    ] {
        let json = common::document(&format!(
            r#""assets": [{{ "id": "a", "kind": "image", "path": "assets/a.png" }}],
               "tracks": [{{ "id": "v1", "kind": "video", "clips": [
                   {{ "id": "c", "asset": "a", "start": 0, "duration": 30 {looks} }}] }}]"#
        ));
        assert!(Project::from_json(&json).is_err(), "{looks} is refused");
    }
    let words: Vec<&str> = [Blend::Normal, Blend::Add, Blend::Screen, Blend::Multiply]
        .iter()
        .map(|blend| blend.as_str())
        .collect();
    assert_eq!(words, ["normal", "add", "screen", "multiply"]);
}
