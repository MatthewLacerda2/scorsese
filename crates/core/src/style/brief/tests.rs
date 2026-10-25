use super::{SCRIPT_FILE, Start, StartError, read_guide, recipe};
use crate::style::{Platform, style};
use crate::{Fps, Project};

#[test]
fn nothing_chosen_writes_nothing() {
    let start = Start::new(None, None).expect("an empty start");
    assert_eq!(start.brief(), None);

    let dir = crate::scratch::fresh("scorsese-brief-nothing", "");
    let mut project = Project::create(&dir, None, Fps::THIRTY, crate::Ratio::WIDE).expect("create");
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
    let dir = crate::scratch::fresh("scorsese-brief-both", "");
    let mut project = Project::create(&dir, None, Fps::THIRTY, crate::Ratio::WIDE).expect("create");
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
    assert!(
        text.contains("upright") && text.contains("faces in the middle"),
        "{text}"
    );
    assert!(text.contains(style("kinetic_type").expect("in the library").prompt));
    assert!(text.contains("propose the script"), "{text}");
    assert!(text.contains("storyboard"), "{text}");
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
        !text.contains("## Style") && !text.contains("faces in the middle"),
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

/// A project started without a style still reaches its recipe (#1531): the
/// section a brief would write, under the id `guide styles` is asked for.
#[test]
fn a_style_or_a_placement_id_is_the_recipe_a_brief_would_write() {
    for style in crate::style::STYLES {
        let said = recipe(style.id).expect("every style has its recipe");
        let brief = Start::new(None, Some(style.id)).expect("a style").brief();
        assert!(brief.expect("a brief").contains(&said), "{}", style.id);
        assert!(said.contains(style.prompt), "{}", style.id);
    }
    for platform in Platform::ALL {
        let said = recipe(platform.id()).expect("every placement has its section");
        let brief = Start::new(Some(platform), None)
            .expect("a platform")
            .brief();
        assert!(brief.expect("a brief").contains(&said), "{platform}");
    }
    let tiktok_ad = recipe("tiktok_ad").expect("a placement");
    assert!(
        tiktok_ad.contains("1080x1920") && tiktok_ad.contains("paid ad"),
        "{tiktok_ad}"
    );
    assert_eq!(recipe("vlog"), None);
}

#[test]
fn the_styles_guide_answers_an_id_with_its_recipe_and_the_rest_from_the_page() {
    let page = |name: &str, section: Option<&str>| match section {
        Some("Whiteboard") => Ok(format!("{name}: the page's section")),
        _ => Err(format!("guide {name}: no such heading")),
    };
    assert_eq!(
        read_guide("styles", Some("kinetic_type"), page),
        Ok(recipe("kinetic_type").unwrap())
    );
    assert_eq!(
        read_guide("styles", Some("Whiteboard"), page).as_deref(),
        Ok("styles: the page's section")
    );
    // Another guide's sections are its own, even one spelled like an id.
    assert!(read_guide("pages", Some("kinetic_type"), page).is_err());
    // A refusal names the ids too, so the next call can use one.
    let refused = read_guide("styles", Some("vlog"), page).expect_err("not a heading or an id");
    assert!(
        refused.contains("no such heading")
            && refused.contains("kinetic_type")
            && refused.contains("tiktok_ad"),
        "{refused}"
    );
    let refused = read_guide("pages", Some("vlog"), page).expect_err("not a heading");
    assert!(!refused.contains("kinetic_type"), "{refused}");
}
