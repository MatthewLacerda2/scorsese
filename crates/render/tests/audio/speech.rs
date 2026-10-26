//! A recording's speech as the Whisper server is sent it (#1549): a WAV,
//! mono, sixteen kilohertz, sixteen bits — whatever the file it came out of.

use scorsese_render::audio::speech;

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

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

    let wav = speech(&tools, &film).expect("a film with sound has speech to send");

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
    let error = speech(&tools, &film).unwrap_err().to_string();
    assert!(error.contains("no sound"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}
