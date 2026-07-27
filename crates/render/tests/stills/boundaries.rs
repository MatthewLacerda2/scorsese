//! The default: one still per stretch, on the frame the stretch begins.

use crate::Rendered;

/// If extraction reached for the frame *after* each boundary, `frame-00030`
/// would still be blue — the shot lasts a second. So the boundary is checked
/// against the frame before it as well, which is where an off-by-one is the
/// difference between red and blue.
#[test]
fn stills_land_on_the_boundaries_the_description_names() {
    let fixture = Rendered::new("stills-boundaries");

    assert_eq!(fixture.description.boundaries(), [0, 30]);
    let written = fixture.write(&fixture.description.boundaries());

    let indices: Vec<u64> = written.iter().map(|still| still.frame).collect();
    assert_eq!(indices, [0, 30]);
    assert_eq!(fixture.colour(&written[0]), "red");
    assert_eq!(fixture.colour(&written[1]), "blue");

    let before = fixture.write(&[29]);
    assert_eq!(
        fixture.colour(&before[0]),
        "red",
        "frame 29 is the last of the first shot; frame 30 is the first of the second"
    );
}

/// A directory listing is in playback order, which is what makes a folder of
/// stills readable as a contact sheet rather than a bag of files.
#[test]
fn stills_are_named_after_the_frame_they_came_from() {
    let fixture = Rendered::new("stills-names");
    let written = fixture.write(&[0, 30]);

    let names: Vec<String> = written
        .iter()
        .map(|still| {
            still
                .path
                .file_name()
                .expect("a still has a file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, ["frame-00000.png", "frame-00030.png"]);
}
