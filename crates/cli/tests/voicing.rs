//! `scorsese cut-to-voice`, end to end: the document afterwards is the whole
//! of what it does.

#[path = "common/mod.rs"]
mod common;

use common::{documents, reload, run_in};

/// Two titles over two unrealised lines, cut long, and a sound under the
/// second title.
fn narrated(label: &str) -> std::path::PathBuf {
    documents::written(
        label,
        &[
            documents::assets::text("one", "ONE"),
            documents::assets::text("two", "TWO"),
            documents::assets::narration("vo"),
            documents::assets::raw("hit", "audio", "wav"),
        ],
        &[
            documents::track(
                "v1",
                "video",
                &[
                    documents::lasting("t1", "one", 0, 300),
                    documents::lasting("t2", "two", 300, 300),
                ],
            ),
            documents::track(
                "vo",
                "audio",
                &[
                    documents::lasting("l1", "vo", 60, 90),
                    documents::lasting("l2", "vo", 400, 60),
                ],
            ),
            documents::track("sfx", "audio", &[documents::lasting("fx", "hit", 320, 10)]),
        ],
    )
}

/// Where clip `id` is now: its start and its end.
fn at(dir: &std::path::Path, id: &str) -> (u64, u64) {
    let project = reload(dir);
    let (_, clip) = project
        .clips()
        .find(|(_, clip)| clip.id.as_str() == id)
        .expect("the clip is still in the project");
    (clip.start.get(), clip.end().get())
}

#[test]
fn each_scene_ends_the_gap_after_its_line_and_its_rider_follows() {
    let dir = narrated("voice");
    run_in(
        &dir,
        &["cut-to-voice", "--scene", "l1=t1", "--scene", "l2=t2+fx@0"],
    )
    .ok()
    .says("2 scene(s) cut to the voice: 5.40s, was 20.00s");
    // Neither line has word timings, so each scene ends 0.2 s (6 frames) after
    // its audio: 90 + 6 = 96, then 96 + 60 + 6 = 162.
    assert_eq!(at(&dir, "t1"), (0, 96));
    assert_eq!(at(&dir, "l2"), (96, 156));
    assert_eq!(at(&dir, "t2"), (96, 162));
    // Twenty frames into its scene before, twenty after.
    assert_eq!(at(&dir, "fx"), (116, 126));
}

#[test]
fn a_scene_that_is_not_a_scene_is_refused_before_anything_is_read() {
    let dir = narrated("voice-refused");
    let before = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let run = run_in(&dir, &["cut-to-voice", "--scene", "l1"]);
    run.says("LINE=VISUAL");
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).unwrap(),
        before
    );
}
