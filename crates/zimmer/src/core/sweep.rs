//! sweep — a cutoff moved from outside the note, as the per-sample base
//! [`super::tracks::cutoff_track`] multiplies.

/// How often a moving base cutoff is read, in sample-frames — about 0.7 ms.
///
/// A curve is a handful of points read through an easing; reading it at every
/// sample of every note costs more than the filter does. Between two reads the
/// value is a straight line, and over 32 samples the difference between that
/// and the curve is far below anything a filter's response can show.
const SWEEP_STEP: usize = 32;

/// `sweep`, a cutoff in Hz at each sample-frame of a note, read every
/// [`SWEEP_STEP`] frames and joined by straight lines into `n` of them.
pub(crate) fn swept(sweep: &dyn Fn(usize) -> f32, n: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(n);
    let mut from = sweep(0);
    let mut at = 0;
    while at < n {
        let next = (at + SWEEP_STEP).min(n.saturating_sub(1).max(at + 1));
        let to = sweep(next);
        let span = (next - at) as f32;
        for k in 0..(next - at).min(n - at) {
            out.push(from + (to - from) * k as f32 / span);
        }
        (from, at) = (to, next);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_line_is_read_back_exactly_at_every_frame() {
        let line = |frame: usize| 100.0 + 2.0 * frame as f32;
        for n in [1, 2, 31, 32, 33, 100, 1000] {
            let swept = swept(&line, n);
            assert_eq!(swept.len(), n, "{n} frames asked for");
            for (frame, value) in swept.iter().enumerate() {
                assert!(
                    (value - line(frame)).abs() < 1e-3,
                    "frame {frame} of {n}: {value}"
                );
            }
        }
    }

    #[test]
    fn a_constant_is_handed_back_bit_for_bit() {
        assert!(swept(&|_| 440.0, 500).iter().all(|v| *v == 440.0));
        assert!(swept(&|_| 440.0, 0).is_empty());
    }
}
