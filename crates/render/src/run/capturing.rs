//! A render with a page in it says it is capturing the page, counting the page
//! frames as they are drawn — and one whose page is already in `cache/` does
//! not say it at all (#1055).
//!
//! Beside `progressed` for the same reason: [`Progress::seen`] and
//! [`Progress::captured`] exist for tests in this crate alone. These need the
//! pinned browser, as the page pipeline tests do.

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};

use crate::progress::{Phase, Progress};
use crate::{FrameRange, RenderSettings, Renderer, Resolution, Tools};

/// How long the page is on screen, in frames: one second.
const LENGTH: u64 = 30;

/// A page that changes every frame, on screen for [`LENGTH`] frames, in a
/// fresh directory handed back with it.
fn fixture() -> (Project, std::path::PathBuf) {
    let root = crate::scratch::fresh("scorsese-capturing", "");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("pages")).expect("create the fixture directory");
    let page = r#"<!doctype html><body style="margin:0"><div id="b"
style="width:50%;height:100%;background:#36c"></div><script>
const start = performance.now();
const tick = (now) => { b.style.opacity = ((now - start) / 1000) % 1;
  requestAnimationFrame(tick); };
requestAnimationFrame(tick);
</script></body>"#;
    std::fs::write(root.join("pages/p.html"), page).expect("write the page");
    let asset = Asset::imported(
        AssetId::new("p"),
        AssetKind::Html,
        ProjectPath::new("pages/p.html"),
    );
    let clip = Clip::new(
        ClipId::new("c1"),
        AssetId::new("p"),
        Frames(0),
        Frames(LENGTH),
    );
    let project = Project {
        assets: vec![asset],
        tracks: vec![Track {
            clips: vec![clip],
            ..Track::new(TrackId::new("v1"), TrackKind::Video)
        }],
        ..Project::new("capturing", Fps::THIRTY)
    };
    (project, root)
}

/// Renders `project` once into `root`, and hands back every phase it passed
/// through, each once, with the readout.
fn rendered(tools: &Tools, project: &Project, root: &std::path::Path) -> (Vec<Phase>, Progress) {
    let progress = Progress::new();
    let raster = Resolution::new(32, 32).expect("a legal raster");
    Renderer::new(tools, RenderSettings::new(raster, Fps::THIRTY))
        .with_progress(progress.clone())
        .render(project, root, FrameRange::ALL, &root.join("out.mp4"))
        .expect("the page renders");
    let mut phases: Vec<Phase> = progress.seen().iter().map(|r| r.phase).collect();
    phases.dedup();
    (phases, progress)
}

#[test]
fn a_page_is_captured_in_a_phase_of_its_own_and_only_while_it_is_not_cached() {
    let tools = crate::testing::tools();
    let (project, root) = fixture();

    let (phases, progress) = rendered(&tools, &project, &root);
    assert_eq!(
        phases,
        [
            Phase::Preparing,
            Phase::Capturing,
            Phase::Mixing,
            Phase::Drawing,
            Phase::Finishing,
            Phase::Done
        ]
    );
    let capturing = progress
        .seen()
        .into_iter()
        .find(|reading| reading.phase == Phase::Capturing)
        .expect("a capturing reading");
    assert_eq!(capturing.done, 0);
    assert!(capturing.of >= LENGTH, "every frame shown: {capturing:?}");
    assert_eq!(
        progress.captured(),
        capturing.of,
        "each drawn frame counted"
    );

    let (again, _) = rendered(&tools, &project, &root);
    std::fs::remove_dir_all(&root).ok();
    assert!(!again.contains(&Phase::Capturing), "cached: {again:?}");
}
