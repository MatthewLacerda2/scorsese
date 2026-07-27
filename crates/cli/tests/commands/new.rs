//! `scorsese new` — the first thing anyone runs, and until now the only one of
//! these that had never been asserted about.

use scorsese_core::{ASSETS_DIR, CACHE_DIR, Fps, GENERATED_DIR, PROJECT_FILE_NAME, SCHEMA_VERSION};

use crate::common::{holds, new_at, reload, scratch, temp_dir};

/// The directory's own name without its extension, which is what the project
/// is called when nobody says otherwise.
fn stem(dir: &std::path::Path) -> String {
    dir.file_stem()
        .and_then(|stem| stem.to_str())
        .expect("the scratch directory has a name")
        .to_owned()
}

#[test]
fn a_new_project_is_a_directory_of_the_shape_the_format_documents() {
    let dir = scratch("new-shape");
    new_at(&dir, &[]).ok();

    for entry in [PROJECT_FILE_NAME, ASSETS_DIR, GENERATED_DIR, CACHE_DIR] {
        assert!(holds(&dir, entry), "a new project has no `{entry}`");
    }
    let project = reload(&dir);
    assert_eq!(project.schema_version, SCHEMA_VERSION);
    assert!(project.assets.is_empty(), "a new project owns no media");
    assert!(project.tracks.is_empty(), "and has nothing on a timeline");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_name_defaults_to_the_directorys_own_stem() {
    // `teaser.scor` holding a project called `teaser` is what anyone omitting
    // the flag meant — extension dropped, and nothing invented.
    let dir = scratch("named-after-me");
    new_at(&dir, &[]).ok();

    assert_eq!(reload(&dir).name, stem(&dir));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_name_flag_is_what_wins_when_there_is_one() {
    let dir = scratch("named-by-flag");
    new_at(&dir, &["--name", "A Boat At Sea"]).ok();

    assert_eq!(reload(&dir).name, "A Boat At Sea");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_timeline_grid_is_chosen_here_and_written_into_the_document() {
    // 29.97 as the exact rational it is. Every clip and keyframe time in the
    // project will be counted on this grid, which is why it is not a default
    // anything can quietly change later.
    let dir = scratch("ntsc");
    new_at(&dir, &["--fps", "30000/1001"]).ok();

    let fps = reload(&dir).timeline_fps;
    assert_eq!((fps.num(), fps.den()), (30_000, 1_001));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn thirty_is_the_grid_when_nobody_names_one() {
    let dir = scratch("default-grid");
    new_at(&dir, &[]).ok();

    assert_eq!(reload(&dir).timeline_fps, Fps::THIRTY);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn creating_a_project_where_one_already_is_refuses_and_leaves_it_alone() {
    // The failure that matters: an agent re-running its own setup step must
    // not be able to blank the edit it made yesterday.
    let dir = scratch("twice");
    new_at(&dir, &["--name", "the original"]).ok();
    let again = new_at(&dir, &["--name", "the replacement"]);

    assert!(
        again.failed,
        "a second `new` must not succeed:\n{}",
        again.output
    );
    assert_eq!(
        reload(&dir).name,
        "the original",
        "the project on disk was overwritten"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_directory_that_exists_but_holds_no_project_is_filled_in() {
    // Someone made the directory first, or cloned an empty one. There is no
    // project in it, so there is nothing to protect and nothing to refuse.
    let dir = temp_dir("existing-empty");
    new_at(&dir, &[]).ok();

    assert!(holds(&dir, PROJECT_FILE_NAME));
    assert_eq!(reload(&dir).name, stem(&dir));
    std::fs::remove_dir_all(&dir).ok();
}
