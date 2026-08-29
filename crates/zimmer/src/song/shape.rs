//! Turning a rendered song into one of the length that was asked for.
//!
//! Two steps, in this order, both after the master limiter:
//!
//! 1. **Length.** Truncate or pad the buffer to the target. A truncation is
//!    always faded over a few milliseconds — a buffer cut at an arbitrary sample
//!    ends mid-waveform, and that is a click, which is more audible than
//!    anything the fade removes.
//! 2. **Fades.** The recipe's own, over whatever length step 1 settled on.
//!
//! Fades come after limiting rather than before because a fade is a decision
//! about the level of the whole piece: limiting afterwards would drag the tail
//! back up and undo it.

use super::Song;
use super::timing::{Fade, Fit, FitMode, Tail};
use crate::core::RATE;
use crate::stereo::Stereo;

/// How long a cut is faded over so it does not click, in seconds.
///
/// Short enough to be inaudible as a fade, long enough to be inaudible as an
/// edge — about a wavelength at the bottom of hearing.
pub(super) const SEAM: f32 = 0.02;

/// Applies `tail`, `fit` and `fade` to a rendered buffer, in that order.
///
/// `arrangement_end` is where the last beat falls, which is the only length
/// the buffer itself cannot tell you: past that point everything is ring-out.
///
/// Length and level are both properties of the *piece*, so every step here is
/// the same step on each channel — a fade that reached one side sooner than
/// the other would be a pan nobody asked for.
pub(crate) fn shape(song: &Song, buf: &mut Stereo, arrangement_end: usize) {
    if song.tail() == Tail::Exact {
        resize(buf, arrangement_end);
    }
    if let Some(fit) = song.fit {
        resize(buf, samples(fit.seconds));
    }
    let fade = song.fade.unwrap_or_default();
    if !fade.is_silent_about_everything() {
        buf.each(|channel| apply_fade(channel, fade));
    }
}

/// The tempo and arrangement this song has to be rendered at to come out at
/// its target length, and the number of passes that takes.
///
/// `Loop` and `Once` keep the written tempo and change how many times the
/// arrangement plays; `Stretch` keeps one whole number of passes and changes
/// the tempo. The caller renders the result and then calls [`shape`], which is
/// what actually lands it on the target sample.
pub(crate) fn plan(song: &Song) -> (f32, u32) {
    let Some(fit) = song.fit else {
        return (song.bpm, 1);
    };
    let once = song.arrangement_beats() * song.beat_seconds();
    if once <= 0.0 {
        return (song.bpm, 1);
    }
    match fit.mode {
        // Ceil, so the buffer is at least as long as the target and `shape`
        // only ever has to cut. Padding a loop with silence would be a gap in
        // the middle of a bed.
        FitMode::Loop => (song.bpm, passes(fit.seconds / once, f32::ceil)),
        FitMode::Once => (song.bpm, 1),
        FitMode::Stretch => {
            let passes = passes(fit.seconds / once, f32::round);
            (stretched_bpm(song, fit, passes), passes)
        }
    }
}

/// How far off the written tempo `stretch` would have to go, as a fraction.
///
/// Its own function because the refusal wants the number: "it would need
/// 137 bpm" is actionable, and "it does not fit" is not.
pub(crate) fn stretch_ratio(song: &Song, fit: Fit) -> Option<f32> {
    let once = song.arrangement_beats() * song.beat_seconds();
    if once <= 0.0 || fit.mode != FitMode::Stretch {
        return None;
    }
    let passes = passes(fit.seconds / once, f32::round);
    Some(stretched_bpm(song, fit, passes) / song.bpm - 1.0)
}

/// The tempo that lands `passes` whole arrangements exactly on the target.
fn stretched_bpm(song: &Song, fit: Fit, passes: u32) -> f32 {
    song.arrangement_beats() * passes as f32 * 60.0 / fit.seconds
}

/// A pass count of at least one, rounded the way the caller asked.
fn passes(exact: f32, round: fn(f32) -> f32) -> u32 {
    let counted = round(exact);
    if counted.is_finite() && counted >= 1.0 {
        counted as u32
    } else {
        1
    }
}

/// Samples in `seconds`, rounded to nearest — the one place a target length
/// becomes a sample count, so `fit` and the tests agree on what 43 s is.
pub(crate) fn samples(seconds: f32) -> usize {
    (seconds * RATE).round().max(0.0) as usize
}

/// Cuts or pads `buf` to exactly `wanted` samples, fading a cut so it does not
/// click.
fn resize(buf: &mut Stereo, wanted: usize) {
    let cut = buf.frames() > wanted;
    buf.resize(wanted);
    if cut {
        buf.each(|channel| fade_out(channel, samples(SEAM)));
    }
}

/// The recipe's own fades, over the buffer as it now stands.
fn apply_fade(buf: &mut [f32], fade: Fade) {
    let rising = samples(fade.in_seconds).min(buf.len());
    for (index, sample) in buf[..rising].iter_mut().enumerate() {
        *sample *= index as f32 / rising as f32;
    }
    fade_out(buf, samples(fade.out_seconds));
}

/// Fades the last `length` samples of `buf` down to silence.
fn fade_out(buf: &mut [f32], length: usize) {
    let length = length.min(buf.len());
    if length == 0 {
        return;
    }
    let start = buf.len() - length;
    for (index, sample) in buf[start..].iter_mut().enumerate() {
        *sample *= 1.0 - (index as f32 / length as f32);
    }
}

/// The seam, by which side of it a length falls on.
///
/// [`resize`] is the one place here that behaves differently depending on a
/// comparison, and the case it has to get right is the boring one: a target
/// that is exactly the length already rendered. Fading there would put a
/// twenty-millisecond dip on the end of a song that was never cut — audible,
/// and invisible to every test that asks how long the result is.
#[cfg(test)]
mod tests {
    use super::*;

    /// A signal that is 1.0 all the way through, so a fade is the only thing
    /// that can make any sample anything else.
    fn flat(frames: usize) -> Stereo {
        Stereo::centred(vec![1.0; frames])
    }

    /// The seam, in samples — long enough that a cut has room to fade.
    fn seam() -> usize {
        samples(SEAM)
    }

    #[test]
    fn a_cut_is_faded_so_it_does_not_click() {
        let mut buf = flat(seam() * 4);
        resize(&mut buf, seam() * 2);
        assert_eq!(buf.frames(), seam() * 2);
        assert_eq!(buf.l[0], 1.0, "the front of it is untouched");
        assert!(buf.l[seam() * 2 - 1] < 0.01, "and the seam runs to silence");
        assert_eq!(buf.l, buf.r, "both sides fade together");
    }

    #[test]
    fn a_target_that_is_already_the_length_changes_nothing() {
        let mut buf = flat(seam() * 4);
        resize(&mut buf, seam() * 4);
        assert_eq!(buf, flat(seam() * 4), "nothing was cut, so nothing fades");
    }

    #[test]
    fn a_longer_target_is_padded_with_silence_and_not_faded() {
        let mut buf = flat(seam());
        resize(&mut buf, seam() * 2);
        assert_eq!(buf.frames(), seam() * 2);
        assert!(
            buf.l[..seam()].iter().all(|s| *s == 1.0),
            "the music is left alone"
        );
        assert!(buf.l[seam()..].iter().all(|s| *s == 0.0), "and padded");
    }
}
