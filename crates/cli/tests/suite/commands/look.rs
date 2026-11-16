//! `scorsese look` — a sheet of a video file, written beside it.
//!
//! With only the file it glances at the whole of it (#1563), as the `look`
//! tool does: frames chosen for it, and the transcript beside the file said
//! when there is one. `--from`, `--to` or `--frames` choose the moments.

use std::path::Path;

use crate::common::media::{colour_video, tools};
use crate::common::{Run, documents, run_in};

fn look(dir: &Path, arguments: &[&str]) -> Run {
    let mut all = vec!["look"];
    all.extend_from_slice(arguments);
    run_in(dir, &all)
}

#[test]
fn a_bare_look_glances_and_reads_the_transcript_beside_the_file() {
    let dir = documents::project("look-glance", &[], &[]);
    colour_video(&tools(), &dir, "shot", "teal", 40);
    let file = dir.join("assets/shot.mp4");
    let file = file.to_str().expect("a UTF-8 path");

    let bare = look(&dir, &[file]).ok();
    bare.says("across the whole of");
    bare.says("It has no transcript.");
    bare.silent_about("--from");
    assert!(dir.join("assets/shot.sheet.png").is_file());

    std::fs::write(
        dir.join("assets/shot.mp4.words.json"),
        r#"{"words":[{"text":"Hi","start":3.0,"end":3.2},{"text":"all.","start":3.3,"end":3.6}]}"#,
    )
    .expect("the test setup holds");
    look(&dir, &[file]).ok().says("0:03 Hi all.");

    let walked = look(&dir, &[file, "--from", "0"]).ok();
    walked.says("More to see: --from 25");
    std::fs::remove_dir_all(&dir).ok();
}
