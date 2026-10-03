//! A render publishes how far it has got, in order, as it goes (#697).
//!
//! The same tiny real render `cancelled` stops part way through, read through
//! [`Progress::seen`] — every reading the render published, in order — rather
//! than by a thread sampling it, which would see whatever the machine happened
//! to schedule.

use scorsese_core::Fps;

use crate::progress::{Phase, Progress, Reading};
use crate::{Cancel, FrameRange, RenderSettings, Renderer, Resolution, Tools};

use super::cancelled::{LENGTH, fixture};

/// Renders the cancel fixture under `cancel`, publishing to a fresh readout,
/// and hands back every reading it published.
fn readings(cancel: Cancel, label: &str) -> Vec<Reading> {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH");
    let (project, root) = fixture(&tools, label);
    let progress = Progress::new();
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let _ = Renderer::new(&tools, RenderSettings::new(raster, Fps::THIRTY))
        .with_cancel(cancel)
        .with_progress(progress.clone())
        .render(&project, &root, FrameRange::ALL, &root.join("out.mp4"));
    std::fs::remove_dir_all(&root).ok();
    progress.seen()
}

#[test]
fn a_render_walks_the_phases_in_order_and_counts_every_frame() {
    let seen = readings(Cancel::new(), "progress-whole");
    let phases: Vec<Phase> = seen.iter().map(|reading| reading.phase).collect();
    assert!(
        phases.windows(2).all(|pair| pair[0] <= pair[1]),
        "{phases:?}"
    );
    let mut visited = phases.clone();
    visited.dedup();
    assert_eq!(
        visited,
        [
            Phase::Preparing,
            Phase::Mixing,
            Phase::Drawing,
            Phase::Finishing,
            Phase::Done
        ]
    );

    let drawn: Vec<u64> = seen
        .iter()
        .filter(|reading| reading.phase == Phase::Drawing)
        .map(|reading| reading.done)
        .collect();
    assert_eq!(drawn, (0..=LENGTH).collect::<Vec<_>>(), "one step a frame");
    let first_mix = seen
        .iter()
        .find(|reading| reading.phase == Phase::Mixing)
        .expect("a mixing reading");
    assert_eq!(first_mix.of, LENGTH, "the length is known before the mix");

    let last = seen.last().expect("at least one reading");
    assert_eq!((last.done, last.of, last.percent()), (LENGTH, LENGTH, 100));
}

#[test]
fn a_cancelled_render_stays_where_it_stopped() {
    // Ten frames encoded and the eleventh refused, as `cancelled` counts it.
    let seen = readings(Cancel::after(12), "progress-mid");
    let last = seen.last().expect("at least one reading");
    assert_eq!(
        *last,
        Reading {
            phase: Phase::Drawing,
            done: 10,
            of: LENGTH
        }
    );
    assert_eq!(last.percent(), 11);
}
