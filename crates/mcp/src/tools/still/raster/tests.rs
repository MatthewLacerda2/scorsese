use super::*;

/// A project whose only video track holds `clips` over the assets `assets`.
fn edit(assets: &str, clips: &str) -> Project {
    serde_json::from_str(&format!(
        r#"{{ "schema_version": {}, "name": "t", "timeline_fps": {{ "num": 30, "den": 1 }},
             "assets": [ {{ "id": "title", "kind": "text", "text": "T" }}, {assets} ],
             "tracks": [ {{ "id": "v1", "kind": "video", "clips": [ {clips} ] }} ] }}"#,
        scorsese_core::SCHEMA_VERSION
    ))
    .expect("a project")
}

fn raster(asked: Option<&str>, project: &Project, budget: u64) -> (u32, u32, Option<String>) {
    let (resolution, from) = choose(asked, project, budget).expect("a raster");
    (
        resolution.width(),
        resolution.height(),
        from.map(str::to_owned),
    )
}

const TALL_SHOT: &str = r#"{ "id": "shot", "kind": "generated_video", "prompt": "a", "state": "sketch", "video": { "aspect": "9:16" } }"#;

#[test]
fn a_landscape_or_shapeless_edit_keeps_the_old_default() {
    let titles = edit(
        r#"{ "id": "t2", "kind": "text", "text": "U" }"#,
        r#"{ "id": "c", "asset": "title", "start": 0, "duration": 30 }"#,
    );
    assert_eq!(raster(None, &titles, FRAME), (1280, 720, None));
    assert_eq!(raster(None, &titles, CELL), (640, 360, None));
}

#[test]
fn a_vertical_edit_gets_a_vertical_preview_of_the_same_budget() {
    let tall = edit(
        TALL_SHOT,
        r#"{ "id": "c", "asset": "title", "start": 0, "duration": 30 },
           { "id": "d", "asset": "shot", "start": 30, "duration": 30 }"#,
    );
    assert_eq!(raster(None, &tall, FRAME), (720, 1280, Some("shot".into())));
    assert_eq!(raster(None, &tall, CELL), (360, 640, Some("shot".into())));
}

#[test]
fn a_probed_source_gives_its_own_shape() {
    let square = edit(
        r#"{ "id": "photo", "kind": "image", "path": "assets/p.png",
             "media": { "width": 1080, "height": 1080 } }"#,
        r#"{ "id": "c", "asset": "photo", "start": 0, "duration": 30 }"#,
    );
    assert_eq!(
        raster(None, &square, FRAME),
        (960, 960, Some("photo".into()))
    );
}

#[test]
fn an_explicit_resolution_still_wins_and_an_aspect_is_spent_at_the_budget() {
    let tall = edit(
        TALL_SHOT,
        r#"{ "id": "d", "asset": "shot", "start": 0, "duration": 30 }"#,
    );
    assert_eq!(raster(Some("1920x1080"), &tall, FRAME), (1920, 1080, None));
    assert_eq!(raster(Some("16:9"), &tall, FRAME), (1280, 720, None));
    assert_eq!(raster(Some("9:16"), &tall, CELL), (360, 640, None));
}

#[test]
fn a_ribbon_is_held_to_four_to_one() {
    let (width, height, _) = raster(Some("1:100"), &edit(TALL_SHOT, ""), FRAME);
    assert_eq!((width, height), (480, 1920));
}

#[test]
fn a_malformed_aspect_is_refused_saying_how_to_write_one() {
    for bad in ["9:0", "x:16", "9:16:1"] {
        let problem = choose(Some(bad), &edit(TALL_SHOT, ""), FRAME).expect_err(bad);
        assert!(problem.contains("like 9:16"), "{problem}");
    }
}

/// A probe that measured nothing is no shape, so the next clip decides — here
/// a generated still, by the aspect it was asked for.
#[test]
fn an_empty_probe_is_passed_over_for_a_generated_stills_aspect() {
    for (width, height) in [(0, 1080), (1080, 0)] {
        let feed = edit(
            &format!(
                r#"{{ "id": "blank", "kind": "image", "path": "assets/b.png",
                     "media": {{ "width": {width}, "height": {height} }} }},
                   {{ "id": "card", "kind": "generated_image", "prompt": "a", "state": "sketch",
                     "image": {{ "aspect": "4:5" }} }}"#
            ),
            r#"{ "id": "c", "asset": "blank", "start": 0, "duration": 30 },
               { "id": "d", "asset": "card", "start": 30, "duration": 30 }"#,
        );
        assert_eq!(raster(None, &feed, FRAME), (858, 1074, Some("card".into())));
    }
}
