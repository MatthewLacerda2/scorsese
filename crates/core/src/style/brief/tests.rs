use super::{SCRIPT_FILE, Start, StartError};
use crate::style::{Platform, style};
use crate::{Fps, Project};

/// A fresh, empty directory for one test.
fn scratch(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("scorsese-brief-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn nothing_chosen_writes_nothing() {
    let start = Start::new(None, None).expect("an empty start");
    assert_eq!(start.brief(), None);

    let dir = scratch("nothing");
    let mut project = Project::create(&dir, None, Fps::THIRTY).expect("create");
    assert_eq!(
        start.write(&dir, &mut project).expect("nothing to write"),
        None
    );
    assert_eq!(project.script, None);
    assert!(!dir.join(SCRIPT_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_platform_and_a_style_are_written_into_the_script() {
    let start = Start::new(Some(Platform::TiktokAd), Some("kinetic_type")).expect("suits");
    let dir = scratch("both");
    let mut project = Project::create(&dir, None, Fps::THIRTY).expect("create");
    let script = start.write(&dir, &mut project).expect("write");

    assert_eq!(
        script.as_ref().map(ToString::to_string).as_deref(),
        Some(SCRIPT_FILE)
    );
    let reloaded = Project::load(&dir).expect("load");
    assert_eq!(reloaded.script, script);
    let text = std::fs::read_to_string(dir.join(SCRIPT_FILE)).expect("read");
    assert!(text.contains("--platform tiktok_ad"), "{text}");
    assert!(text.contains("1080x1920"), "{text}");
    assert!(text.contains("paid ad"), "{text}");
    assert!(text.contains(style("kinetic_type").expect("in the library").prompt));
    assert!(text.contains("propose the script"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn either_alone_still_carries_the_next_step() {
    let platform = Start::new(Some(Platform::Youtube), None).expect("a platform");
    let text = platform.brief().expect("a brief");
    assert!(
        text.contains("1920x1080") && !text.contains("paid ad"),
        "{text}"
    );
    assert!(
        !text.contains("## Style") && !text.contains("upright"),
        "{text}"
    );
    assert!(text.contains("propose the script"), "{text}");

    let styled = Start::new(None, Some("whiteboard")).expect("a style");
    let text = styled.brief().expect("a brief");
    assert!(!text.contains("## Platform"), "{text}");
    assert!(text.contains("propose the script"), "{text}");
}

#[test]
fn an_unknown_style_is_refused_with_the_library() {
    let error = Start::new(None, Some("vlog")).expect_err("not in the library");
    assert_eq!(error, StartError::UnknownStyle("vlog".to_owned()));
    assert!(error.to_string().contains("kinetic_type"), "{error}");
}

#[test]
fn a_style_not_made_for_the_platform_is_refused() {
    let whiteboard = style("whiteboard").expect("in the library");
    assert!(!whiteboard.suits(Platform::TiktokAd));
    let error = Start::new(Some(Platform::TiktokAd), Some("whiteboard")).expect_err("unsuited");
    let message = error.to_string();
    assert!(
        message.contains("kinetic_type") && !message.contains("whiteboard,"),
        "{message}"
    );
}
