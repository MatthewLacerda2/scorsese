//! A frame written out as a PNG lands as a file of its own, whatever was at
//! its name.

use scorsese_render::Frame;
use scorsese_render::frames;

use super::{raster, tools};
use crate::common::scratch;

/// A `.scor` received from someone else may carry a link where a contact sheet
/// is written (`cache/stock/sheet.png`, #1737): the sheet replaces the link,
/// and the file it named is left as it was.
#[cfg(unix)]
#[test]
fn a_link_at_a_pngs_name_is_replaced_never_written_through() {
    let tools = tools();
    let dir = scratch::dir("png-planted");
    let outside = scratch::dir("png-planted-outside");
    let victim = outside.join("notes.txt");
    std::fs::write(&victim, "the victim's own bytes").expect("a victim");
    let out = dir.join("sheet.png");
    std::os::unix::fs::symlink(&victim, &out).expect("a link");

    let frame = Frame::black(raster());
    frames::write_png(&tools, &out, &frame).expect("the sheet is written");

    assert_eq!(
        std::fs::read(&victim).expect("the victim"),
        b"the victim's own bytes",
        "the link's target was written through"
    );
    let landed = std::fs::symlink_metadata(&out).expect("the sheet");
    assert!(landed.is_file(), "the link was not replaced");
    let back = frames::read_png(&tools, &out, raster()).expect("the sheet reads back");
    assert_eq!(back.bytes(), frame.bytes());
    assert_eq!(
        std::fs::read_dir(&dir).expect("the folder").count(),
        1,
        "no scratch is left beside the sheet"
    );
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&outside).ok();
}

/// A PNG that cannot be written is said of the name asked for, never the
/// scratch name it was drawn under, which is gone by then.
#[test]
fn a_failed_png_names_the_file_asked_for() {
    let tools = tools();
    let dir = scratch::dir("png-refused");
    // ffmpeg picks no encoder for this extension.
    let out = dir.join("sheet.nothing");

    let refused = frames::write_png(&tools, &out, &Frame::black(raster()));

    let said = refused.expect_err("no encoder").to_string();
    assert!(said.contains(&out.display().to_string()), "{said}");
    assert!(!said.contains(".drawing"), "{said}");
    std::fs::remove_dir_all(&dir).ok();
}
