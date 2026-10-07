//! What changing the size of a transparent picture must do: keep a white edge
//! white at partial coverage, leave a picture at its own size alone, and make
//! whatever falls outside the picture transparent.

use scorsese_compositor::{BYTES_PER_PIXEL, CpuCompositor, Frame, Resample, Resolution};

fn size(width: u32, height: u32) -> Resolution {
    Resolution::source(width, height).expect("a legal size")
}

/// A white square in the middle of a transparent surround whose colour is
/// black — what every exporter leaves behind a fully transparent pixel.
fn badge() -> Frame {
    let mut frame = Frame::black(size(32, 32));
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let (x, y) = (index % 32, index / 32);
        let inside = (8..24).contains(&x) && (8..24).contains(&y);
        let value = if inside { u8::MAX } else { 0 };
        pixel.copy_from_slice(&[value, value, value, value]);
    }
    frame
}

fn pixels(frame: &Frame) -> impl Iterator<Item = &[u8]> {
    frame.bytes().chunks_exact(BYTES_PER_PIXEL)
}

/// The rim straight-alpha resampling draws: every pixel with any coverage at
/// all is white, whatever its alpha came out as.
#[test]
fn a_white_edge_stays_white_at_partial_coverage() {
    for scaled in [size(64, 64), size(13, 13)] {
        let resample = Resample::new(size(32, 32), scaled, scaled, (0, 0));
        let mut out = Frame::black(scaled);
        CpuCompositor::new().resample(&resample, &badge(), &mut out);
        let partial = pixels(&out).filter(|p| p[3] > 0 && p[3] < u8::MAX).count();
        assert!(
            partial > 0,
            "{scaled}: the edge was resampled, so some is partial"
        );
        for pixel in pixels(&out).filter(|p| p[3] > 0) {
            assert_eq!(&pixel[..3], &[255, 255, 255], "{scaled}: {pixel:?}");
        }
    }
}

/// Catmull-Rom is exact at whole pixels, so the same size is the same bytes.
#[test]
fn a_picture_at_its_own_size_comes_out_as_it_went_in() {
    let resample = Resample::new(size(32, 32), size(32, 32), size(32, 32), (0, 0));
    let mut out = Frame::black(size(32, 32));
    CpuCompositor::new().resample(&resample, &badge(), &mut out);
    assert_eq!(out, badge());
}

/// A window larger than the picture — a padded fit — is transparent outside
/// it, and a window inside a larger picture — a fill — is the part asked for.
#[test]
fn a_window_keeps_the_part_asked_for_and_nothing_else() {
    let padded = Resample::new(size(32, 32), size(32, 32), size(48, 32), (-8, 0));
    let mut out = Frame::black(size(48, 32));
    CpuCompositor::new().resample(&padded, &badge(), &mut out);
    let at = |frame: &Frame, x: usize, y: usize| {
        let i = (y * frame.resolution().width() as usize + x) * BYTES_PER_PIXEL;
        frame.bytes()[i..i + BYTES_PER_PIXEL].to_vec()
    };
    assert_eq!(at(&out, 2, 16), [0, 0, 0, 0], "padding is transparent");
    assert_eq!(
        at(&out, 24, 16),
        [255, 255, 255, 255],
        "the badge moved over by 8"
    );

    let filled = Resample::new(size(32, 32), size(64, 64), size(16, 16), (24, 24));
    let mut out = Frame::black(size(16, 16));
    CpuCompositor::new().resample(&filled, &badge(), &mut out);
    assert!(
        pixels(&out).all(|p| p == [255, 255, 255, 255]),
        "the centre is all badge"
    );
}

/// The weights are worked out once and the scratch is the compositor's, so
/// the second frame through is the first frame's answer — nothing left over
/// from the last one, or from a larger resample through the same compositor,
/// leaks into the next.
#[test]
fn every_frame_is_resampled_from_its_own_pixels() {
    let resample = Resample::new(size(32, 32), size(20, 20), size(20, 20), (0, 0));
    let larger = Resample::new(size(32, 32), size(64, 64), size(64, 64), (0, 0));
    let (mut first, mut again) = (Frame::black(size(20, 20)), Frame::black(size(20, 20)));
    let mut compositor = CpuCompositor::new();
    compositor.resample(&resample, &badge(), &mut first);
    compositor.resample(&resample, &Frame::black(size(32, 32)), &mut again);
    compositor.resample(&larger, &badge(), &mut Frame::black(size(64, 64)));
    compositor.resample(&resample, &badge(), &mut again);
    assert_eq!(first, again);
}

/// A field of one colour at partial coverage resamples to that colour at that
/// coverage: weighting by alpha and dividing it back out must cancel, at any
/// size and any alpha.
#[test]
fn a_translucent_field_keeps_its_colour_and_its_coverage() {
    for alpha in [1, 64, 128, 200] {
        let mut field = Frame::black(size(30, 30));
        for pixel in field.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&[200, 100, 50, alpha]);
        }
        for scaled in [size(45, 45), size(11, 11)] {
            let resample = Resample::new(size(30, 30), scaled, scaled, (0, 0));
            let mut out = Frame::black(scaled);
            CpuCompositor::new().resample(&resample, &field, &mut out);
            for pixel in pixels(&out) {
                assert_eq!(pixel[3], alpha, "alpha {alpha} at {scaled}");
                for (got, want) in pixel[..3].iter().zip([200_u8, 100, 50]) {
                    assert!(
                        got.abs_diff(want) <= 1,
                        "alpha {alpha} at {scaled}: {pixel:?}"
                    );
                }
            }
        }
    }
}

/// Catmull-Rom reproduces a straight line exactly, so a ramp doubled in size
/// is the same ramp sampled twice as often — the property that tells this
/// cubic from a blurrier or a ringing one. Twelve levels a pixel, so every
/// sample doubling lands on lies on a whole level and must be met exactly.
#[test]
fn a_ramp_grows_into_the_same_ramp() {
    let mut ramp = Frame::black(size(20, 1));
    for (x, pixel) in ramp
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let level = (x * 12) as u8;
        pixel.copy_from_slice(&[level, level, level, u8::MAX]);
    }
    let resample = Resample::new(size(20, 1), size(40, 1), size(40, 1), (0, 0));
    let mut out = Frame::black(size(40, 1));
    CpuCompositor::new().resample(&resample, &ramp, &mut out);
    // Away from the ends, where the filter is cut short and renormalised.
    for (x, pixel) in pixels(&out).enumerate().take(36).skip(4) {
        let want = 6 * x - 3;
        assert_eq!(usize::from(pixel[0]), want, "x {x}: {pixel:?}");
    }
}

/// A hard edge stays hard: the cubic's negative outer lobes overshoot either
/// side of a step, which is what keeps a page's text crisp when it shrinks
/// rather than the grey smear an all-positive filter makes of it.
#[test]
fn a_hard_edge_overshoots_rather_than_smears() {
    let mut step = Frame::black(size(24, 1));
    for (x, pixel) in step
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let level = if x < 12 { 50 } else { 200 };
        pixel.copy_from_slice(&[level, level, level, u8::MAX]);
    }
    for scaled in [size(48, 1), size(10, 1)] {
        let resample = Resample::new(size(24, 1), scaled, scaled, (0, 0));
        let mut out = Frame::black(scaled);
        CpuCompositor::new().resample(&resample, &step, &mut out);
        let levels: Vec<u8> = pixels(&out).map(|pixel| pixel[0]).collect();
        let (low, high) = (levels.iter().min(), levels.iter().max());
        assert!(low < Some(&50) && high > Some(&200), "{scaled}: {levels:?}");
    }
}
