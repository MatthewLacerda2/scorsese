//! A page timed to a spoken word by `scorsese.words` (#811): told when a
//! narration says it, in the page's own seconds, through the clip that plays
//! the line.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

use scorsese_core::words::{Word, Words};
use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::page::Chrome;
use scorsese_render::{Frame, Renderer};

use crate::common::ffmpeg::{fixture_dir, generate, tools};
use crate::common::prompts::generated_asset;
use crate::common::{audio_track, clip, project, video_track};
use crate::{BLUE, RED, assert_colour, colour_asset, settings};

/// Blue over the whole frame while the narration `vo` says *gradient*.
const PAGE: &str = "<div id='lit' style='position:absolute;inset:0'></div>\
    <style>@keyframes on { from, to { background: #00f } }</style>\
    <script>const word = scorsese.words['vo/gradient'];\
    lit.style.animation = `on ${word.end - word.start}s linear ${word.start}s`;</script>";

fn top_left(frame: &Frame) -> (u8, u8, u8) {
    let bytes = frame.bytes();
    (bytes[0], bytes[1], bytes[2])
}

#[test]
fn a_page_lights_on_the_word_where_the_clip_plays_it() {
    let tools = tools();
    let dir = fixture_dir("told-words");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/lit.html"), PAGE).expect("a page");
    let line = generated_asset("line", AssetKind::GeneratedAudio);
    let audio = line.path.clone().expect("a generated line has a file");
    generate(
        &tools,
        &audio.resolve(&dir),
        &["-f", "lavfi", "-i", "sine=duration=2"],
    );
    // *gradient* is said from 1.0 s to 1.5 s into the line.
    let said = Words {
        words: vec![
            Word {
                text: "The".into(),
                start: 0.5,
                end: 0.9,
            },
            Word {
                text: "gradient,".into(),
                start: 1.0,
                end: 1.5,
            },
        ],
    };
    std::fs::write(Words::beside(&audio).resolve(&dir), said.to_json()).expect("timings");
    let page = Asset::imported(
        AssetId::new("lit"),
        AssetKind::Html,
        ProjectPath::new("pages/lit.html"),
    );
    // The line opens 0.5 s in at timeline frame 15 (0.5 s), so *gradient* is
    // heard from 1.0 s to 1.5 s of the timeline: frames 30 to 45.
    let mut vo = clip("vo", "line", 15, 45);
    vo.source_in = Frames(15);
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 2), page, line],
        vec![
            video_track("v1", vec![clip("shot", "red", 0, 60)]),
            video_track("v2", vec![clip("page", "lit", 0, 60)]),
            audio_track("a1", vec![vo]),
        ],
    );
    let chrome = Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)");
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY)).with_chrome(chrome);
    let at = |frame: u64| {
        let still = renderer
            .still(&project, &dir, Frames(frame))
            .expect("composes");
        top_left(&still)
    };
    assert_colour(at(37), BLUE, "while *gradient* is said");
    assert_colour(at(24), RED, "while *The* is");
    assert_colour(at(50), RED, "after it");
    std::fs::remove_dir_all(&dir).ok();
}
