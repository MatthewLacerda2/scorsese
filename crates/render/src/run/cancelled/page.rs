//! A render stopped while its page is being captured: it stops within about a
//! second, and leaves neither a file nor a half-captured page in `cache/`
//! (#1088). Needs the pinned browser, as the page pipeline tests do.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};

use crate::progress::Progress;
use crate::{Cancel, FrameRange, RenderError, RenderSettings, Renderer, Resolution, Tools};

/// How long the page is on screen: twenty seconds, far longer than the
/// capture is let run.
const LENGTH: u64 = 600;

/// A page that changes every frame, in a fresh directory handed back with it.
fn fixture() -> (Project, PathBuf) {
    let root = crate::scratch::fresh("scorsese-uncaptured", "");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("pages")).expect("create the fixture directory");
    let page = r#"<!doctype html><body style="margin:0"><div id="b"
style="width:50%;height:100%;background:#c63"></div><script>
const tick = (now) => { b.style.opacity = (now / 1000) % 1; requestAnimationFrame(tick); };
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
        ..Project::new("uncaptured", Fps::THIRTY)
    };
    (project, root)
}

/// Every file under `dir`, however deep.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .map(|entry| entry.expect("an entry").path())
        .flat_map(|path| {
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

#[test]
fn a_cancel_mid_capture_stops_within_a_second_and_keeps_nothing_of_it() {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH");
    let (project, root) = fixture();
    let out = root.join("out.mp4");
    let (cancel, progress) = (Cancel::new(), Progress::new());
    // Tripped from another thread once the browser has drawn a few frames,
    // the way a person stopping a render would.
    let asked = std::thread::spawn({
        let (cancel, progress) = (cancel.clone(), progress.clone());
        move || {
            let given_up = Instant::now() + Duration::from_secs(120);
            while progress.captured() < 5 && Instant::now() < given_up {
                std::thread::sleep(Duration::from_millis(5));
            }
            cancel.cancel();
            Instant::now()
        }
    });
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let outcome = Renderer::new(&tools, RenderSettings::new(raster, Fps::THIRTY))
        .with_cancel(cancel)
        .with_progress(progress.clone())
        .render(&project, &root, FrameRange::ALL, &out);
    let took = asked.join().expect("the asking thread").elapsed();
    let captured = progress.captured();
    let kept: Vec<PathBuf> = files_under(&root.join("cache"))
        .into_iter()
        .filter(|file| file.extension().is_some_and(|ext| ext == "mkv"))
        .collect();
    let left = (out.exists(), root.join(".out.mp4.partial").exists());
    std::fs::remove_dir_all(&root).ok();

    assert!(
        matches!(
            outcome,
            Err(RenderError::Cancelled {
                written: 0,
                of: LENGTH
            })
        ),
        "{outcome:?}"
    );
    assert!(captured < LENGTH, "it stopped part way: {captured} frames");
    assert!(
        took < Duration::from_secs(3),
        "stopped {took:?} after asked"
    );
    assert!(kept.is_empty(), "a stopped capture kept {kept:?}");
    assert_eq!(left, (false, false), "a stopped render left its file");
}
