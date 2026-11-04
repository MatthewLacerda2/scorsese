//! The `page_captions` fixture is what captioning writes (#1550): its page
//! and its captions track are a re-run's output, so a change to the page look
//! fails here first and re-blessing the fixture is the deliberate next step.

use std::path::Path;

use scorsese_core::captions::caption;
use scorsese_core::{PropertyPath, TrackId};
use scorsese_render::audio::path;
use scorsese_render::picture::caption::Choices;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/page_captions");
const PAGE: &str = "pages/caption-page-captions.html";
const WORDS: &str = "assets/voice.wav.words.json";

#[test]
fn a_rerun_writes_the_fixture_unchanged() {
    let fixture = Path::new(FIXTURE);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("page-captions-rerun");
    let _ = std::fs::remove_dir_all(&dir);
    for file in ["project.json", PAGE, WORDS] {
        let target = dir.join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(fixture.join(file), &target).unwrap();
    }
    let mut project = scorsese_core::Project::load(&dir).unwrap();
    let before = project.clone();
    let mut asked = Choices::default().captioning(TrackId::new("captions"), 30.0);
    asked.volume = Some(PropertyPath::new(path::VOLUME));
    caption(&mut project, &dir, &asked).unwrap();

    assert_eq!(project, before, "the captions track a re-run writes");
    let written = std::fs::read_to_string(dir.join(PAGE)).unwrap();
    let carried = std::fs::read_to_string(fixture.join(PAGE)).unwrap();
    assert!(
        written == carried,
        "the page look changed: re-run `scorsese caption` in the fixture and \
         re-bless `page_captions` (docs/golden-renders.md)"
    );
    std::fs::remove_dir_all(&dir).ok();
}
