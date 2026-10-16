//! `Project::create`, which refuses anything already in the directory, for
//! every caller at once (#1288).

use scorsese_core::{Fps, PROJECT_DIRS, PROJECT_FILE_NAME, Project, SaveError};

use crate::common::{new_project, temp_project_dir};

#[test]
fn an_empty_directory_is_laid_out() {
    let dir = temp_project_dir("create-empty");
    Project::create(&dir, None, Fps::THIRTY, scorsese_core::Ratio::WIDE)
        .expect("an empty directory is free");
    assert!(dir.join(PROJECT_FILE_NAME).is_file());
    for folder in PROJECT_DIRS {
        assert!(dir.join(folder).is_dir(), "no `{folder}`");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_directory_holding_anything_is_refused_untouched() {
    let dir = temp_project_dir("create-occupied");
    std::fs::write(dir.join("holiday.mp4"), b"not a project").expect("something in it");
    let refused = Project::create(&dir, None, Fps::THIRTY, scorsese_core::Ratio::WIDE);
    assert!(
        matches!(refused, Err(SaveError::NotEmpty { .. })),
        "{refused:?}"
    );
    assert!(!dir.join(PROJECT_FILE_NAME).exists());
    assert!(PROJECT_DIRS.iter().all(|folder| !dir.join(folder).exists()));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_is_refused_as_one() {
    let (dir, _) = new_project("create-twice");
    let refused = Project::create(&dir, None, Fps::THIRTY, scorsese_core::Ratio::WIDE);
    assert!(
        matches!(refused, Err(SaveError::AlreadyAProject { .. })),
        "{refused:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
