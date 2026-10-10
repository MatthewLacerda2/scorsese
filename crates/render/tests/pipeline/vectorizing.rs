//! `vectorize` (#999): a project's picture traced into `pages/<name>.svg`.
//!
//! The tracing itself is held by `trace`'s own tests on pixels made in
//! memory; these hold the half around it — finding the picture, decoding it
//! through ffmpeg whatever its format, writing beside the pages — and the
//! refusals that write nothing.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState};
use scorsese_render::trace::{TraceError, Traced, Tracing, Vectorized, vectorize};

use crate::common::ffmpeg::{fixture_dir, generate_asset, tools};
use crate::common::project;

/// A dark-ringed red square on white: flat art, made by ffmpeg.
const BADGE: &[&str] = &[
    "-f",
    "lavfi",
    "-i",
    "color=white:s=160x120,drawbox=x=40:y=20:w=80:h=80:color=0x141418:t=fill,\
     drawbox=x=50:y=30:w=60:h=60:color=0xc8281e:t=fill",
    "-frames:v",
    "1",
];

#[test]
fn a_picture_is_traced_into_a_file_beside_the_pages() {
    let tools = tools();
    let dir = fixture_dir("vectorize");
    let picture = generate_asset(&tools, &dir, "badge", AssetKind::Image, BADGE);
    let project = project(vec![picture], Vec::new());

    let written = vectorize(&tools, &project, &dir, "badge", "seal", Tracing::default())
        .expect("a picture traces");
    assert_eq!(written.path, "pages/seal.svg");
    assert_eq!((written.traced.width, written.traced.height), (160, 120));
    assert_eq!((written.traced.strokes, written.traced.shapes), (2, 1));
    let svg = std::fs::read_to_string(dir.join("pages/seal.svg")).expect("the file is there");
    assert_eq!(svg, written.traced.svg);
    assert!(svg.contains("<g class=\"drawing\" id=\"seal\">"), "{svg}");
    assert!(
        written
            .summary()
            .starts_with("pages/seal.svg — 160x120, 2 pen strokes")
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn what_cannot_be_traced_writes_nothing() {
    let tools = tools();
    let dir = fixture_dir("vectorize-refused");
    let picture = generate_asset(&tools, &dir, "badge", AssetKind::Image, BADGE);
    let mut sketch = Asset::sketch(
        AssetId::new("sketch"),
        AssetKind::GeneratedImage,
        "a flat illustration",
    );
    sketch.state = Some(GenerationState::Sketch);
    let words = Asset::text(AssetId::new("words"), "hello");
    let project = project(vec![picture, sketch, words], Vec::new());
    let refused = |asset: &str, name: &str| {
        vectorize(&tools, &project, &dir, asset, name, Tracing::default()).expect_err("refused")
    };

    assert!(matches!(refused("nope", "x"), TraceError::Unknown(_)));
    assert!(matches!(refused("words", "x"), TraceError::NotAPicture(_)));
    assert!(matches!(
        refused("sketch", "x"),
        TraceError::NotGenerated(_)
    ));
    assert!(matches!(refused("badge", "../x"), TraceError::BadName(_)));
    assert!(!dir.join("pages").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_mosaic_is_called_heavy() {
    let traced = |strokes: usize, shapes: usize| Vectorized {
        traced: Traced {
            svg: String::new(),
            width: 64,
            height: 64,
            colours: vec!["#000000".to_owned()],
            strokes,
            shapes,
        },
        path: "pages/a.svg".to_owned(),
        name: "a".to_owned(),
    };
    assert!(!traced(17, 12).summary().contains("heavy"));
    assert!(traced(80, 3_000).summary().contains("heavy"));
}
