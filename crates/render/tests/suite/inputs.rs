//! A file's name or contents never decide what else ffmpeg opens (#1635).

use crate::common;

use std::path::Path;

use scorsese_compositor::Resolution;
use scorsese_core::ProbeMedia;
use scorsese_render::Ffprobe;
use scorsese_render::frames::extract;
use scorsese_render::testing::colour_video;
use scorsese_render::tools::input;

use common::ffmpeg::tools;
use common::scratch;

/// A list in ffconcat's format, saved as `.mp4`, naming `shot.mp4` beside it.
fn list_named_as_media(dir: &Path) -> std::path::PathBuf {
    colour_video(&tools(), &dir.join("shot.mp4"), "red", "64x64", 1);
    let list = dir.join("list.mp4");
    std::fs::write(&list, "ffconcat version 1.0\nfile shot.mp4\n").expect("the list writes");
    list
}

/// Run where `concat:a.mp4|b.mp4` would open the two files it names, the
/// name opens nothing but itself: here, a file that is not there.
#[cfg(unix)]
#[test]
fn a_name_that_reads_as_a_protocol_opens_only_the_file_so_named() {
    let tools = tools();
    let dir = scratch::dir("protocol-name");
    colour_video(&tools, &dir.join("a.mp4"), "red", "64x64", 1);
    colour_video(&tools, &dir.join("b.mp4"), "red", "64x64", 1);
    let mut probe = tools.ffprobe();
    probe
        .current_dir(&dir)
        .args(["-v", "error", "-show_entries"]);
    probe.args(["stream=width", "-of", "csv=p=0"]);
    let output = input::media(&mut probe, Path::new("concat:a.mp4|b.mp4"))
        .output()
        .expect("ffprobe runs");
    std::fs::remove_dir_all(&dir).ok();
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success() && !said.contains("64"),
        "a.mp4 was opened: {said}"
    );
}

#[test]
fn a_list_named_as_media_is_refused_by_the_probe_as_a_listing() {
    let dir = scratch::dir("probed-list");
    let list = list_named_as_media(&dir);
    let probed = Ffprobe::new(tools()).probe(&list);
    std::fs::remove_dir_all(&dir).ok();
    let error = probed.expect_err("a list is not media");
    assert!(error.listing, "{error}");
    assert!(error.message.contains("playlist"), "{error}");
}

#[test]
fn an_hls_playlist_named_as_media_is_refused_by_the_probe() {
    let dir = scratch::dir("probed-playlist");
    colour_video(&tools(), &dir.join("shot.mp4"), "red", "64x64", 1);
    let playlist = dir.join("playlist.mp4");
    let lines = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXTINF:1,\nshot.mp4\n#EXT-X-ENDLIST\n";
    std::fs::write(&playlist, lines).expect("the playlist writes");
    let probed = Ffprobe::new(tools()).probe(&playlist);
    std::fs::remove_dir_all(&dir).ok();
    assert!(probed.is_err(), "{probed:?}");
}

/// A name that is a numbered pattern is a sequence of other files to the
/// demuxer that reads a picture, never the one picture.
#[test]
fn a_picture_named_as_a_numbered_pattern_is_refused_by_the_probe() {
    let tools = tools();
    let dir = scratch::dir("probed-pattern");
    scorsese_render::testing::still(&tools, &dir.join("card1.jpg"), "red", "16x16");
    std::fs::copy(dir.join("card1.jpg"), dir.join("card%d.jpg")).expect("the copy writes");
    let probed = Ffprobe::new(tools).probe(&dir.join("card%d.jpg"));
    std::fs::remove_dir_all(&dir).ok();
    let error = probed.expect_err("a pattern is not a picture");
    assert!(error.listing, "{error}");
}

#[test]
fn a_list_named_as_media_placed_in_a_project_fails_to_decode() {
    let dir = scratch::dir("decoded-list");
    let list = list_named_as_media(&dir);
    let decoded = extract(&tools(), &list, 0, Resolution::new(64, 64).expect("a size"));
    std::fs::remove_dir_all(&dir).ok();
    assert!(decoded.is_err(), "the list's shot was decoded");
}

#[test]
fn media_still_probes_through_the_whitelist() {
    let tools = tools();
    let dir = scratch::dir("probed-media");
    let file = dir.join("shot.mp4");
    colour_video(&tools, &file, "red", "64x64", 1);
    let probed = Ffprobe::new(tools).probe(&file);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(probed.expect("media probes").width, Some(64));
}
