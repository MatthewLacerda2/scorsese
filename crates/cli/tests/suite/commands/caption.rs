//! `scorsese caption` choosing the rim's colour, as `caption_narration` can
//! (#1288).

use scorsese_core::words::{Word, Words};
use scorsese_core::{ProjectPath, Rgba};

use crate::common::{documents, reload, run_in};

/// A project with one generated line, its word timings beside it.
fn spoken(label: &str) -> std::path::PathBuf {
    let dir = documents::written(
        label,
        &[documents::assets::generated("vo", "audio", "mp3")],
        &[documents::track(
            "narration",
            "audio",
            &[documents::lasting("v1", "vo", 60, 60)],
        )],
    );
    std::fs::create_dir_all(dir.join("generated")).expect("generated/");
    std::fs::write(dir.join("generated/vo.mp3"), b"MP3").expect("the line");
    let word = |text: &str, start, end| Word {
        text: text.into(),
        start,
        end,
    };
    let timed = Words {
        words: vec![word("Stop.", 0.2, 0.6), word("Read.", 0.9, 1.4)],
    };
    let beside = Words::beside(&ProjectPath::new("generated/vo.mp3"));
    std::fs::write(
        beside.resolve(&dir).expect("a project path"),
        timed.to_json(),
    )
    .expect("its timings");
    dir
}

/// The rim every caption in the project was set with.
fn strokes(dir: &std::path::Path) -> Vec<Option<Rgba>> {
    reload(dir)
        .assets
        .iter()
        .filter_map(|asset| asset.style.as_ref())
        .map(|style| style.stroke)
        .collect()
}

#[test]
fn the_rim_is_a_colour_or_none_and_no_stroke_is_none() {
    let dir = spoken("caption-stroke");
    let red = Rgba::opaque(255, 0, 0);
    let run = run_in(&dir, &["caption", "--stroke", "#ff0000"]);
    assert!(!run.failed, "{}", run.output);
    assert_eq!(strokes(&dir), vec![Some(red), Some(red)]);

    for off in [&["--stroke", "none"][..], &["--no-stroke"]] {
        let run = run_in(&dir, &[&["caption"][..], off].concat());
        assert!(!run.failed, "{}", run.output);
        assert_eq!(strokes(&dir), vec![None, None], "{off:?}");
    }

    let run = run_in(&dir, &["caption", "--stroke", "red"]);
    assert!(run.failed, "a word that is not a colour is refused");
    std::fs::remove_dir_all(&dir).ok();
}
