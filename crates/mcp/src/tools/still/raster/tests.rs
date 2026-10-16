use scorsese_core::{Fps, Ratio};

use super::*;

fn framed(aspect: Ratio) -> Project {
    Project {
        aspect,
        ..Project::new("t", Fps::THIRTY)
    }
}

fn raster(asked: Option<&str>, project: &Project, budget: Budget) -> (u32, u32) {
    let (resolution, _) = choose(asked, project, budget).expect("a raster");
    (resolution.width(), resolution.height())
}

#[test]
fn a_landscape_project_keeps_the_old_default() {
    let wide = framed(Ratio::WIDE);
    assert_eq!(raster(None, &wide, Budget::Preview), (1280, 720));
    assert_eq!(raster(None, &wide, Budget::Cell), (640, 360));
}

#[test]
fn a_vertical_project_gets_a_vertical_preview_of_the_same_budget() {
    let tall = framed(Ratio::TALL);
    assert_eq!(raster(None, &tall, Budget::Preview), (720, 1280));
    assert_eq!(raster(None, &tall, Budget::Cell), (360, 640));
    let (_, said) = choose(None, &tall, Budget::Preview).expect("a raster");
    assert_eq!(said, " (the project's 9:16 frame)");
}

#[test]
fn an_explicit_resolution_still_wins_and_an_aspect_is_spent_at_the_budget() {
    let tall = framed(Ratio::TALL);
    assert_eq!(
        raster(Some("1920x1080"), &tall, Budget::Preview),
        (1920, 1080)
    );
    assert_eq!(raster(Some("16:9"), &tall, Budget::Preview), (1280, 720));
    assert_eq!(raster(Some("9:16"), &tall, Budget::Cell), (360, 640));
    let (_, said) = choose(Some("16:9"), &tall, Budget::Preview).expect("a raster");
    assert_eq!(said, "");
}

#[test]
fn a_malformed_or_ribbon_aspect_is_refused_saying_how_to_write_one() {
    for bad in ["9:0", "x:16", "9:16:1", "1:100"] {
        let problem = choose(Some(bad), &framed(Ratio::TALL), Budget::Preview).expect_err(bad);
        assert!(problem.contains("like 9:16"), "{problem}");
    }
}
