//! `scorsese sequence` — a folder of frames in as one image sequence, then
//! retimed.

use scorsese_core::AssetKind;

use super::Outside;
use crate::common::media::{generate, tools};
use crate::common::{reload, run_in};

#[test]
fn a_folder_of_frames_imports_as_one_sequence_and_can_be_retimed() {
    let outside = Outside::new("sequence");
    std::fs::create_dir_all(outside.elsewhere.join("spin")).expect("a folder of frames");
    // Three colours, because the same picture twice is one asset.
    for (name, colour) in [("f_10", "blue"), ("f_1", "red"), ("f_2", "lime")] {
        let file = outside.elsewhere.join(format!("spin/{name}.png"));
        let source = format!("color=c={colour}:s=64x64");
        generate(
            &tools(),
            &file,
            &["-f", "lavfi", "-i", &source, "-frames:v", "1"],
        );
    }
    let folder = outside.elsewhere.join("spin");
    let folder = folder.to_str().expect("a utf-8 fixture path");
    let run = run_in(
        &outside.project,
        &["sequence", "import", folder, "--hold", "2", "--loop"],
    )
    .ok();
    run.says("spin — image_sequence: 3 stills × 2 frame(s) = 6 frames, looping");
    run.says("gap: 7 missing between f_2.png and f_10.png");

    let project = reload(&outside.project);
    let spin = project
        .assets
        .iter()
        .find(|asset| asset.kind == AssetKind::ImageSequence)
        .expect("one sequence");
    let stills: Vec<&str> = spin
        .sequence
        .as_ref()
        .expect("its block")
        .stills
        .iter()
        .map(|id| id.as_str())
        .collect();
    assert_eq!(stills, ["spin-f_1", "spin-f_2", "spin-f_10"]);
    assert!(outside.project.join("assets/spin/f_10.png").is_file());

    run_in(
        &outside.project,
        &["sequence", "set", "spin", "--loop", "false"],
    )
    .ok()
    .says("now 3 stills × 2 frame(s) = 6 frames, once");
    run_in(
        &outside.project,
        &["sequence", "set", "spin", "--hold", "0"],
    )
    .says("holds each still for 0 frames");
    outside.clean();
}
