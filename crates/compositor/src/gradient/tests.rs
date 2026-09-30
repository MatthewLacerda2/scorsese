//! The gradient's arithmetic, below the pixels: where along it a stop is read,
//! how translucent stops blend, and the dither's exact values.

use scorsese_core::{Fill, Linear, Rgba, Stop};

use super::*;
use crate::frame::Resolution;

/// A left-to-right gradient over a 100-pixel box, to sample by `t`.
fn across(stops: &[(Rgba, f64)]) -> Gradient {
    let fill = Fill::Linear(Linear {
        angle: 90.0,
        stops: stops
            .iter()
            .map(|&(color, at)| Stop::new(color, at))
            .collect(),
    });
    Gradient::new(&fill, (0.0, 0.0, 100.0, 10.0)).expect("a gradient")
}

const RED: Rgba = Rgba::opaque(255, 0, 0);
const BLUE: Rgba = Rgba::opaque(0, 0, 255);

#[test]
fn before_the_first_stop_is_the_first_colour_exactly() {
    let ramp = across(&[(RED, 0.2), (BLUE, 0.6)]);
    assert_eq!(ramp.sample(0.0), [255.0, 0.0, 0.0, 255.0]);
    assert_eq!(ramp.sample(0.9), [0.0, 0.0, 255.0, 255.0]);
    assert_eq!(ramp.sample(f64::NAN), [255.0, 0.0, 0.0, 255.0]);
}

/// Measured from the stop it leaves, over the distance to the next one.
#[test]
fn between_stops_the_colour_is_measured_from_the_stop_it_leaves() {
    let ramp = across(&[(RED, 0.2), (BLUE, 0.6)]);
    let [r, _, b, _] = ramp.sample(0.3);
    assert!(
        (r - 191.25).abs() < 1e-9 && (b - 63.75).abs() < 1e-9,
        "{r} {b}"
    );
}

/// Two stops at one offset are a hard edge, and the later one wins from it.
#[test]
fn a_hard_edge_switches_at_its_offset() {
    let ramp = across(&[(RED, 0.0), (RED, 0.5), (BLUE, 0.5), (BLUE, 1.0)]);
    assert_eq!(ramp.sample(0.499), [255.0, 0.0, 0.0, 255.0]);
    assert_eq!(ramp.sample(0.5), [0.0, 0.0, 255.0, 255.0]);
}

/// Premultiplied: red fading to transparent blue stays red on the way, only
/// thinner. Interpolated straight, the middle would be half-transparent purple.
#[test]
fn a_stop_fading_to_transparent_keeps_the_colour_it_fades() {
    let ramp = across(&[(RED, 0.0), (Rgba::new(0, 0, 255, 0), 1.0)]);
    let [r, g, b, a] = straight(ramp.sample(0.5));
    assert!(
        (r - 255.0).abs() < 1e-9 && g == 0.0 && b == 0.0,
        "{r} {g} {b}"
    );
    assert!((a - 127.5).abs() < 1e-9, "{a}");
    let half = Rgba::new(40, 80, 120, 128);
    let flat = across(&[(half, 0.0), (half, 1.0)]);
    let back = straight(flat.sample(0.7)).map(f64::round);
    assert_eq!(back, [40.0, 80.0, 120.0, 128.0]);
}

/// Red, blue and alpha of one pixel of a 64-wide frame.
fn at(frame: &Frame, x: usize, y: usize) -> [u8; 3] {
    let i = (y * 64 + x) * BYTES_PER_PIXEL;
    [frame.bytes()[i], frame.bytes()[i + 2], frame.bytes()[i + 3]]
}

/// A colour asset's layer: the whole raster, left to right.
#[test]
fn a_whole_frame_is_painted_edge_to_edge() {
    let mut frame = Frame::black(Resolution::new(64, 8).expect("a raster"));
    let fill = Fill::Linear(Linear {
        angle: 90.0,
        stops: vec![Stop::new(RED, 0.0), Stop::new(BLUE, 1.0)],
    });
    paint(&mut frame, &fill);
    for y in [0, 7] {
        assert!(
            at(&frame, 0, y)[0] >= 250 && at(&frame, 0, y)[1] <= 5,
            "left {:?}",
            at(&frame, 0, y)
        );
        assert!(
            at(&frame, 63, y)[1] >= 250 && at(&frame, 63, y)[0] <= 5,
            "right {:?}",
            at(&frame, 63, y)
        );
        assert!(
            at(&frame, 32, y)[0].abs_diff(127) <= 4,
            "middle {:?}",
            at(&frame, 32, y)
        );
        assert_eq!(at(&frame, 32, y)[2], 255, "opaque");
    }
    let downwards = Fill::Linear(Linear {
        angle: 180.0,
        stops: vec![Stop::new(RED, 0.0), Stop::new(BLUE, 1.0)],
    });
    paint(&mut frame, &downwards);
    assert!(
        at(&frame, 40, 0)[0] >= 220 && at(&frame, 40, 7)[1] >= 220,
        "top red, bottom blue"
    );
    paint(&mut frame, &Fill::Solid(BLUE));
    assert!(frame.bytes().chunks(4).all(|p| p == [0, 0, 255, 255]));
}

#[test]
fn the_noise_is_centred_and_bounded() {
    let samples: Vec<f64> = (0..256)
        .flat_map(|y| (0..256).map(move |x| noise(x, y)))
        .collect();
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    assert!(mean.abs() < 0.02, "mean {mean}");
    assert!(samples.iter().all(|n| n.abs() <= DITHER));
    let spread = samples.iter().map(|n| n * n).sum::<f64>() / samples.len() as f64;
    assert!((spread.sqrt() - 0.816).abs() < 0.02, "σ {}", spread.sqrt());
}

/// Written down, so "the same picture on every machine" is asserted rather
/// than assumed: a different hash, sum or seed moves these.
#[test]
fn the_noise_is_pinned() {
    let pinned = [(0, 0), (7, 3), (1919, 1079)].map(|(x, y)| noise(x, y));
    assert_eq!(pinned, PINNED);
}

#[test]
fn noise_moves_a_channel_before_it_is_rounded() {
    assert_eq!(
        dithered([100.3, 100.3, 100.3, 255.0], 0.4),
        [101, 101, 101, 255]
    );
    assert_eq!(
        dithered([100.3, 100.3, 100.3, 255.0], -0.4),
        [100, 100, 100, 255]
    );
}

#[test]
fn a_channel_at_either_end_is_never_dithered_off_it() {
    for n in [-2.0, -0.6, 0.6, 2.0] {
        assert_eq!(dithered([0.0, 255.0, 0.0, 255.0], n), [0, 255, 0, 255]);
    }
}

/// `noise` at (0, 0), (7, 3) and (1919, 1079), as this build computed them.
const PINNED: [f64; 3] = [1.0185546537438768, -1.243020769498083, 0.6250865968468335];
