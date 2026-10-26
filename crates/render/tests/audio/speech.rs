//! A recording's speech as the Whisper server is sent it (#1549): a WAV,
//! mono, sixteen kilohertz, sixteen bits — whatever the file it came out of.

use scorsese_render::Cancel;
use scorsese_render::audio::speech;

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

/// Longer than any fixture here.
pub(crate) const LONGEST: f64 = 60.0;

/// A two-second stereo film at 48 kHz, with a tone in its sound.
#[test]
fn a_films_sound_comes_out_as_a_small_mono_wav() {
    let tools = tools();
    let dir = scratch::dir("speech-film");
    let film = dir.join("take.mp4");
    generate(
        &tools,
        &film,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=32x32:d=2:r=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=2",
            "-ac",
            "2",
            "-pix_fmt",
            "yuv420p",
            "-shortest",
        ],
    );

    let wav = speech(&tools, &film, LONGEST, &Cancel::new())
        .expect("a film with sound has speech to send");

    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "mono");
    assert_eq!(
        u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
        16_000
    );
    assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16, "bits a sample");
    let seconds = (wav.len() - 44) as f64 / 32_000.0;
    assert!((seconds - 2.0).abs() < 0.1, "{seconds} s");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_film_with_no_sound_says_so() {
    let tools = tools();
    let dir = scratch::dir("speech-silent-film");
    let film = dir.join("mute.mp4");
    generate(
        &tools,
        &film,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=32x32:d=1:r=30",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let error = speech(&tools, &film, LONGEST, &Cancel::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("no sound"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A film whose sound starts half a second after its picture, as a phone's
/// can: the WAV starts at the film's own zero, silence first, so each word's
/// time is where the render seeks for it.
#[test]
fn sound_that_starts_late_keeps_its_place_on_the_films_clock() {
    let tools = tools();
    let dir = scratch::dir("speech-late");
    let first = dir.join("first.mp4");
    generate(
        &tools,
        &first,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=32x32:d=2:r=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=1",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let late = dir.join("late.mp4");
    let first = first.to_str().expect("a scratch path is text");
    generate(
        &tools,
        &late,
        &[
            "-i",
            first,
            "-itsoffset",
            "0.5",
            "-i",
            first,
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c",
            "copy",
        ],
    );

    let wav = speech(&tools, &late, LONGEST, &Cancel::new()).expect("it has sound");

    let data = wav
        .windows(4)
        .position(|at| at == b"data")
        .expect("a data chunk")
        + 8;
    let samples: Vec<i16> = wav[data..]
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let heard = samples
        .iter()
        .position(|sample| sample.unsigned_abs() > 100)
        .expect("the tone is in it") as f64
        / 16_000.0;
    assert!((heard - 0.5).abs() < 0.05, "the tone starts at {heard} s");
    std::fs::remove_dir_all(&dir).ok();
}
