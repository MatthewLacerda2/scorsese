//! What a tool's picture becomes before the model is sent it (#1435).
//!
//! A tool answers with a PNG, lossless, because a file someone keeps should
//! be. The model gains nothing from that: it downsamples past
//! [`frames::MODEL_EDGE`] on the long edge, and a 1280×720 PNG of a
//! photographic frame was 2–3 MB of base64. The conversation is append-only
//! and replayed whole on every turn (the parent module), so session 8 of
//! project 11 reached 72 MB in twenty turns, and every turn after it was
//! refused for its size. Each picture is therefore sent — and so stored in
//! `chat_turns.messages`, and replayed — as a JPEG at that edge. Nothing the
//! user owns is touched: a still is a preview, and a purchase stays as the
//! vendor sent it.
//!
//! Here rather than in `scorsese_mcp::Part::picture`, because a local MCP
//! client is the user's own agent on the user's own machine, which decides
//! what it does with a lossless frame; the web's conversation is the one this
//! server pays to replay.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scorsese_providers::chat::ResultPart;
use scorsese_render::{Tools, frames};

/// The longest edge of a picture the person attached to a message (#1562):
/// enough to read what it shows, at most 1,369 tokens (⌈w/28⌉ × ⌈h/28⌉,
/// Anthropic's *Vision* page, read 2026-10-25) on every call that replays
/// it, where [`frames::MODEL_EDGE`] costs up to 3,136.
pub(super) const ATTACHED_EDGE: u32 = 1024;

/// The picture the person attached in `file`, as the JPEG the model is sent.
pub(super) fn attached(ffmpeg: &Tools, file: &Path) -> Result<ResultPart, String> {
    let jpeg =
        frames::file_for_a_model(ffmpeg, file, ATTACHED_EDGE).map_err(|error| error.to_string())?;
    Ok(ResultPart::Image {
        media_type: frames::JPEG.to_owned(),
        data: STANDARD.encode(jpeg),
    })
}

/// A base64 picture as the JPEG the model is sent, or a line saying it is
/// not there — never the picture whole, which is what ended a conversation.
pub(super) fn shrunk(ffmpeg: &Tools, picture: &str) -> ResultPart {
    let jpeg = STANDARD
        .decode(picture)
        .map_err(|error| error.to_string())
        .and_then(|bytes| frames::for_a_model(ffmpeg, &bytes).map_err(|error| error.to_string()));
    match jpeg {
        Ok(jpeg) => ResultPart::Image {
            media_type: frames::JPEG.to_owned(),
            data: STANDARD.encode(jpeg),
        },
        Err(why) => {
            tracing::warn!("assistant: shrinking a picture: {why}");
            ResultPart::Text {
                text: "(a picture here could not be prepared to show you; ask for it again)".into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of its own for one test, under the system's temp.
    fn scratch(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scorsese-pictures-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the test setup holds");
        dir
    }

    /// A 1280×720 frame as busy as a photograph, a still's default size: its
    /// PNG is 2.5 MB, as session 8's were.
    fn photographic(ffmpeg: &Tools) -> Vec<u8> {
        let out = ffmpeg
            .ffmpeg()
            .args(["-nostdin", "-v", "error", "-f", "lavfi", "-i"])
            .arg("mandelbrot=s=1280x720,noise=alls=12:allf=t")
            .args(["-frames:v", "1", "-c:v", "png", "-f", "image2pipe", "-"])
            .output()
            .expect("ffmpeg runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    #[test]
    fn a_still_is_sent_as_a_jpeg_a_fraction_of_its_png() {
        let ffmpeg = Tools::discover().expect("ffmpeg is on PATH");
        let png = STANDARD.encode(photographic(&ffmpeg));
        let ResultPart::Image { media_type, data } = shrunk(&ffmpeg, &png) else {
            panic!("the picture is sent");
        };
        assert_eq!(media_type, "image/jpeg");
        assert!(
            data.len() * 8 < png.len(),
            "{} of {}",
            data.len(),
            png.len()
        );
    }

    #[test]
    fn an_attached_picture_is_sent_no_longer_than_its_edge_keeping_its_aspect() {
        let ffmpeg = Tools::discover().expect("ffmpeg is on PATH");
        let scratch = scratch("fitted");
        let png = scratch.join("frame.png");
        std::fs::write(&png, photographic(&ffmpeg)).expect("the test setup holds");
        let ResultPart::Image { media_type, data } =
            attached(&ffmpeg, &png).expect("a picture shrinks")
        else {
            panic!("the picture is sent");
        };
        assert_eq!(media_type, "image/jpeg");
        let sent = scratch.join("sent.jpg");
        std::fs::write(&sent, STANDARD.decode(data).expect("base64")).expect("it writes");
        let probed = ffmpeg
            .ffprobe()
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
            .args(["stream=width,height", "-of", "csv=p=0"])
            .arg(&sent)
            .output()
            .expect("ffprobe runs");
        std::fs::remove_dir_all(&scratch).ok();
        let size = String::from_utf8_lossy(&probed.stdout).trim().to_owned();
        assert_eq!(size, "1024,576", "1280×720 fitted to {ATTACHED_EDGE}");
    }

    /// Piped, a 1600×1200 uncompressed TIFF (a kind the library takes) was
    /// found to hold no picture at all.
    #[test]
    fn a_picture_too_large_to_pipe_is_read_from_its_file_and_shown() {
        let ffmpeg = Tools::discover().expect("ffmpeg is on PATH");
        let out = ffmpeg
            .ffmpeg()
            .args(["-nostdin", "-v", "error", "-f", "lavfi", "-i"])
            .arg("testsrc=s=1600x1200")
            .args(["-frames:v", "1", "-c:v", "tiff", "-compression_algo", "raw"])
            .args(["-f", "image2pipe", "-"])
            .output()
            .expect("ffmpeg runs");
        assert!(out.stdout.len() > 5_000_000, "{}", out.stdout.len());
        let scratch = scratch("tiff");
        let tiff = scratch.join("big.tif");
        std::fs::write(&tiff, out.stdout).expect("the test setup holds");
        let shown = attached(&ffmpeg, &tiff);
        std::fs::remove_dir_all(&scratch).ok();
        assert!(shown.is_ok(), "{shown:?}");
    }

    #[test]
    fn a_picture_that_cannot_be_shrunk_is_said_to_be_missing_not_sent_whole() {
        let ffmpeg = Tools::discover().expect("ffmpeg is on PATH");
        let sent = shrunk(&ffmpeg, &STANDARD.encode(b"not a picture"));
        assert!(matches!(sent, ResultPart::Text { .. }), "{sent:?}");
    }
}
