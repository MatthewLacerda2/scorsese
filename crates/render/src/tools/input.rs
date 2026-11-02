//! Naming a file to ffmpeg or ffprobe as an input: the one way it is done.
//!
//! ffmpeg reads an input's name as a URL when it can — `concat:a|b`,
//! `subfile:`, `http:` each open something other than the file so named — and
//! some formats (an HLS playlist, an ffconcat list) open further inputs named
//! inside the file. On the web app the files are members' uploads, decoded on
//! a shared machine, so a file's name or contents must never decide what else
//! ffmpeg opens (#1635). Every input here is therefore:
//!
//! - read only through the [`PROTOCOLS`] a local file needs,
//! - named `file:<absolute path>`, so no name is read as a protocol, and
//! - for media, held to the [`MEDIA_FORMATS`] an importable file may be, so a
//!   playlist renamed `.mp4` is refused before it opens anything.
//!
//! `make size`'s `ffmpeg-input` rule flags any other `"-i"` in the crates that
//! run ffmpeg.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

/// The protocols an input may be opened through: a file, and a pipe for an
/// input this program writes to ffmpeg's stdin.
pub const PROTOCOLS: &str = "file,pipe";

/// The demuxers a media file scorsese imports may be read with, as ffmpeg
/// names them: every format `scorsese_core::pool::extensions` takes, and the
/// raw streams a misnamed one of those turns out to be. A name matches a
/// demuxer with several (`mov` is `mov,mp4,m4a,3gp,3g2,mj2`).
///
/// Never a format that opens other inputs: `hls`, `dash` and `concat` are
/// what this list keeps out. `image2` reads one picture unless its file's
/// name is a numbered pattern, which an imported name never is
/// (`scorsese_core::pool` keeps a name to letters, digits, `-`, `_` and `.`)
/// and the probe refuses ([`crate::Ffprobe`]).
pub const MEDIA_FORMATS: &str = "mov,matroska,webm,avi,mpeg,mpegts,mpegvideo,m4v,h264,hevc,\
                                 asf,flv,ogg,image2,image2pipe,png_pipe,jpeg_pipe,webp_pipe,\
                                 gif,gif_pipe,bmp_pipe,tiff_pipe,mp3,wav,w64,aac,flac,aiff";

/// The demuxers that open further inputs named inside a file: what a file
/// probed as one of these is (a playlist or a list), and why it is refused.
pub const LISTING_FORMATS: [&str; 3] = ["hls", "dash", "concat"];

/// Adds media `file` to `command` as its next input: through [`PROTOCOLS`]
/// only, held to [`MEDIA_FORMATS`], and named so no part of it reads as a
/// protocol. Input options (`-ss`, `-loop`) go before it, as before any
/// `-i`. Works for ffprobe too, which takes `-i` the same way.
pub fn media<'c>(command: &'c mut Command, file: &Path) -> &'c mut Command {
    command.args(["-format_whitelist", MEDIA_FORMATS]);
    own(command, file)
}

/// Adds `file` to `command` as its next input through [`PROTOCOLS`] only,
/// named so no part of it reads as a protocol, in whatever format the
/// command already forces (`-f f32le` for a mix this program wrote, `-f
/// concat` for a list it wrote). Media from anywhere else takes [`media`].
pub fn own<'c>(command: &'c mut Command, file: &Path) -> &'c mut Command {
    command
        .args(["-protocol_whitelist", PROTOCOLS])
        // lint: allow(ffmpeg-input) the one place a file input is built
        .arg("-i")
        .arg(url(file))
}

/// Adds stdin to `command` as a media input, held to [`MEDIA_FORMATS`]:
/// for a picture this program was handed as bytes rather than a file.
pub fn piped_media(command: &mut Command) -> &mut Command {
    command
        .args(["-format_whitelist", MEDIA_FORMATS])
        .args(["-protocol_whitelist", "pipe"])
        // lint: allow(ffmpeg-input) stdin, named so ffmpeg reads it through `pipe`
        .args(["-i", "pipe:0"])
}

/// `file` as ffmpeg's `file` protocol names it, absolute so it does not
/// depend on where ffmpeg runs. ffmpeg takes everything after `file:`
/// literally, so a `:` or `|` later in the name is only part of it.
pub fn url(file: &Path) -> OsString {
    let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    let mut url = OsString::from("file:");
    url.push(file.as_os_str());
    url
}

#[cfg(test)]
mod tests {
    use super::{MEDIA_FORMATS, PROTOCOLS, media, piped_media, url};
    use std::path::Path;
    use std::process::Command;

    fn arguments(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_media_input_is_whitelisted_and_named_as_an_absolute_file() {
        let mut command = crate::tools::Tools::at("ffmpeg", "ffprobe").ffmpeg();
        media(&mut command, Path::new("concat:a.mp4|b.mp4"));
        let here = std::env::current_dir().unwrap();
        let named = format!("file:{}", here.join("concat:a.mp4|b.mp4").display());
        assert_eq!(
            arguments(&command),
            [
                "-format_whitelist",
                MEDIA_FORMATS,
                "-protocol_whitelist",
                PROTOCOLS,
                // lint: allow(ffmpeg-input) what the helper is expected to write
                "-i",
                &named
            ]
        );
    }

    #[test]
    fn an_absolute_name_is_kept_as_it_is_after_the_protocol() {
        let file = std::env::temp_dir().join("shot.mp4");
        assert_eq!(
            url(&file).to_string_lossy(),
            format!("file:{}", file.display())
        );
    }

    #[test]
    fn stdin_is_read_through_the_pipe_alone() {
        let mut command = crate::tools::Tools::at("ffmpeg", "ffprobe").ffmpeg();
        piped_media(&mut command);
        let arguments = arguments(&command);
        assert_eq!(
            arguments[2..],
            // lint: allow(ffmpeg-input) what the helper is expected to write
            ["-protocol_whitelist", "pipe", "-i", "pipe:0"]
        );
    }

    #[test]
    fn no_format_that_opens_other_inputs_is_media() {
        for listing in super::LISTING_FORMATS {
            assert!(
                !MEDIA_FORMATS.split(',').any(|format| format == listing),
                "{listing}"
            );
        }
    }
}
