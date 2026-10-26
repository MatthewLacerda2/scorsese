//! The frame's shape (#1278): chosen by `new`, changed by `reframe`, and what
//! `describe` measures against when nobody names a resolution.

use scorsese_core::{PROJECT_FILE_NAME, Ratio, SCHEMA_VERSION};

use crate::common::{new_at, new_project, reload, run_in, run_without_tools, scratch};

#[test]
fn a_platform_or_an_aspect_frames_a_new_project() {
    let tall = scratch("new-tiktok");
    new_at(&tall, &["--platform", "tiktok"])
        .ok()
        .says("framed 9:16");
    assert_eq!(reload(&tall).aspect, Ratio::TALL);

    let portrait = scratch("new-portrait");
    new_at(&portrait, &["--platform", "tiktok", "--aspect", "4:5"]).ok();
    assert_eq!(reload(&portrait).aspect, Ratio::new(4, 5).expect("4:5"));

    let plain = new_project("new-plain");
    assert_eq!(reload(&plain).aspect, Ratio::WIDE);
    for dir in [tall, portrait, plain] {
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn reframe_changes_the_shape_and_says_so() {
    let dir = new_project("reframed");
    run_in(&dir, &["reframe", "1080:1920"])
        .ok()
        .says("Reframed from 16:9 to 9:16.");
    assert_eq!(reload(&dir).aspect, Ratio::TALL);
    run_in(&dir, &["reframe", "9:16"])
        .ok()
        .says("already 9:16; nothing changed");
    let refused = run_in(&dir, &["reframe", "8:1"]);
    assert!(refused.failed, "a ribbon is no frame");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn describe_measures_a_vertical_project_against_an_upright_frame() {
    let dir = new_project("describe-tall");
    let document = format!(
        r##"{{ "schema_version": {SCHEMA_VERSION}, "name": "tall",
             "timeline_fps": {{ "num": 30, "den": 1 }}, "aspect": "9:16",
             "assets": [{{ "id": "bed", "kind": "color", "color": "#101820" }}],
             "tracks": [{{ "id": "v1", "kind": "video", "clips": [
                 {{ "id": "c", "asset": "bed", "start": 0, "duration": 30 }}] }}] }}"##
    );
    std::fs::write(dir.join(PROJECT_FILE_NAME), document).expect("write the document");
    run_without_tools(&dir, &["describe", "--at", "0"])
        .ok()
        .says("on a 1080x1920 frame");
    run_without_tools(&dir, &["describe", "--at", "0", "--resolution", "16:9"])
        .ok()
        .says("on a 1920x1080 frame");
    std::fs::remove_dir_all(dir).ok();
}
