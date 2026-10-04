//! An image sequence in the files panel: one row, its stills folded under it
//! (#684), unfolded here so the picture shows both halves of the decision.

use egui_kittest::kittest::Queryable;

use crate::drawing::window;
use crate::fixture::{self, DOCUMENT, Fixture, PIXEL};

/// The whole edit with a three-photo timelapse added, one of whose photos a
/// clip also shows on its own, and one of which is missing from disk — so the
/// unfolded list has a plain row, an "also used on its own" row and a warning.
fn sequenced(label: &str) -> Fixture {
    let plate = r#"{ "id": "plate", "kind": "image", "path": "assets/plate.png",
      "media": { "width": 1, "height": 1 } },"#;
    let stills = (1..=3)
        .map(|n| {
            format!(
                r#"{{ "id": "dawn-{n}", "kind": "image", "path": "assets/dawn-{n}.png",
      "media": {{ "width": 1, "height": 1 }} }},"#
            )
        })
        .collect::<String>();
    let sequence = r#"{ "id": "dawn", "kind": "image_sequence",
      "sequence": { "stills": ["dawn-1", "dawn-2", "dawn-3"], "hold": 2 } },"#;
    let title = r#"{ "id": "c-title", "asset": "title", "start": 300, "duration": 120 }"#;
    let alone = r#", { "id": "c-dawn", "asset": "dawn-2", "start": 420, "duration": 60 }"#;
    assert!(
        DOCUMENT.contains(plate) && DOCUMENT.contains(title),
        "the anchors still match the fixture"
    );
    let document = DOCUMENT
        .replace(plate, &format!("{plate}\n    {stills}\n    {sequence}"))
        .replace(title, &format!("{title}{alone}"));
    let fixture = fixture::write(label, &document);
    for n in [1, 2] {
        std::fs::write(fixture.path().join(format!("assets/dawn-{n}.png")), PIXEL)
            .expect("write a still");
    }
    fixture
}

#[test]
fn a_sequence_unfolded_in_the_files_panel() {
    let project = sequenced("sequence");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.get_by_label("⏵").click();
    // Off the panel, so the pointer does not sit over the arrow in the picture.
    harness.hover_at(egui::pos2(700.0, 780.0));
    harness.run();
    harness.snapshot("a_sequence_unfolded_in_the_files_panel");
}
