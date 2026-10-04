//! Real renders, small enough to run in the suite: a white card on black, a
//! second long, at a raster the size of an icon.

use std::time::{Duration, Instant};

use scorsese_core::{
    Asset, AssetId, Clip, ClipId, Fps, Frames, Geometry, Project, Rgba, Shape, Track, TrackId,
    TrackKind,
};
use scorsese_render::{Phase, RenderSettings, Resolution};

use super::{Ended, Job};

/// Nothing to decode, so the only external thing it needs is the encoder.
fn card() -> Project {
    let card = Geometry::Rectangle {
        width: 0.5,
        height: 0.5,
        radius: 0.0,
    };
    let card = Asset::shape(AssetId::new("card"), Shape::filled(card, Rgba::WHITE));
    let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
    track.clips = vec![Clip::new(
        ClipId::new("c1"),
        AssetId::new("card"),
        Frames(0),
        Frames(30),
    )];
    Project {
        assets: vec![card],
        tracks: vec![track],
        ..Project::new("card", Fps::THIRTY)
    }
}

/// A fresh directory for one test's project and file.
fn directory(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scorsese-app-render-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create the test directory");
    root
}

fn small() -> RenderSettings {
    RenderSettings::new(
        Resolution::new(32, 32).expect("a legal raster"),
        Fps::THIRTY,
    )
}

/// Polls as the window does, until the job answers.
fn wait(job: &mut Job) -> Ended {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(ended) = job.ended() {
            return ended.clone();
        }
        assert!(Instant::now() < deadline, "the render never answered");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_render_runs_to_a_file_and_reads_done() {
    let root = directory("done");
    let out = root.join("card.mp4");
    let mut job = Job::start(&card(), &root, small(), out.clone());
    let ended = wait(&mut job);
    assert!(matches!(ended, Ended::Wrote(_)), "{ended:?}");
    assert!(out.is_file(), "the file is there");
    assert_eq!(job.reading().phase, Phase::Done);
    assert_eq!(job.reading().percent(), 100);
    assert!(!job.stopping());
}

#[test]
fn stop_ends_it_with_no_file_left_behind() {
    let root = directory("stopped");
    let out = root.join("card.mp4");
    let mut job = Job::start(&card(), &root, small(), out.clone());
    job.stop();
    let ended = wait(&mut job);
    assert_eq!(ended, Ended::Stopped);
    assert!(!out.exists(), "a stopped render leaves nothing at the path");
}

#[test]
fn a_render_that_cannot_run_says_why() {
    let root = directory("failed");
    // Nothing on the timeline: there is no film to render.
    let empty = Project::new("empty", Fps::THIRTY);
    let mut job = Job::start(&empty, &root, small(), root.join("empty.mp4"));
    match wait(&mut job) {
        Ended::Failed(why) => assert!(!why.is_empty()),
        other => panic!("expected a failure, got {other:?}"),
    }
}
