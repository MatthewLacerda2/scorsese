//! What changing the size of a transparent picture must do: keep a white edge
//! white at partial coverage, leave a picture at its own size alone, and make
//! whatever falls outside the picture transparent.

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resample, Resolution};

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
        let mut resample = Resample::new(size(32, 32), scaled, scaled, (0, 0));
        let mut out = Frame::black(scaled);
        resample.apply(&badge(), &mut out);
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
    let mut resample = Resample::new(size(32, 32), size(32, 32), size(32, 32), (0, 0));
    let mut out = Frame::black(size(32, 32));
    resample.apply(&badge(), &mut out);
    assert_eq!(out, badge());
}

/// A window larger than the picture — a padded fit — is transparent outside
/// it, and a window inside a larger picture — a fill — is the part asked for.
#[test]
fn a_window_keeps_the_part_asked_for_and_nothing_else() {
    let mut padded = Resample::new(size(32, 32), size(32, 32), size(48, 32), (-8, 0));
    let mut out = Frame::black(size(48, 32));
    padded.apply(&badge(), &mut out);
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

    let mut filled = Resample::new(size(32, 32), size(64, 64), size(16, 16), (24, 24));
    let mut out = Frame::black(size(16, 16));
    filled.apply(&badge(), &mut out);
    assert!(
        pixels(&out).all(|p| p == [255, 255, 255, 255]),
        "the centre is all badge"
    );
}

/// The weights are worked out once, so the second frame through is the first
/// frame's answer — nothing left over from the last one leaks into the next.
#[test]
fn every_frame_is_resampled_from_its_own_pixels() {
    let mut resample = Resample::new(size(32, 32), size(20, 20), size(20, 20), (0, 0));
    let (mut first, mut again) = (Frame::black(size(20, 20)), Frame::black(size(20, 20)));
    resample.apply(&badge(), &mut first);
    resample.apply(&Frame::black(size(32, 32)), &mut again);
    resample.apply(&badge(), &mut again);
    assert_eq!(first, again);
}
