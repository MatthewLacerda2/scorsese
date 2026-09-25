//! The rows with no picture in them: a render delivered as sound only (#505).
//!
//! What these hold is the promise `docs/output-formats.md` makes about them —
//! a file with no video stream, a mix identical to the one the video would
//! carry, and a compositor that is never reached at all — plus the refusals
//! that come with it. The picture rows are `containers.rs`.

use std::path::Path;

use scorsese_core::{AssetKind, Project};
use scorsese_render::{
    Container, FrameRange, OutputFormat, PlanError, RenderError, RenderReport, Renderer, Tools,
};

use crate::common::ffmpeg::{fixture_dir, generate_asset, tools};
use crate::common::{audio_track, clip, project};
use crate::probing::{Delivered, probe};
use crate::{demo, settings};

/// Renders `project` into `container`, handing back the report and the
/// file's path — or the refusal.
fn render(
    tools: &Tools,
    dir: &Path,
    project: &Project,
    container: Container,
) -> (Result<RenderReport, RenderError>, std::path::PathBuf) {
    let out = dir.join(format!("out.{container}"));
    let settings = settings(OutputFormat::defaults_for(container));
    let report = Renderer::new(tools, settings).render(project, dir, FrameRange::ALL, &out);
    (report, out)
}

/// The demo, delivered as `container`, as ffprobe reads it back.
fn heard(label: &str, container: Container) -> (Delivered, RenderReport) {
    let tools = tools();
    let dir = fixture_dir(label);
    let (report, out) = render(&tools, &dir, &demo(&tools, &dir), container);
    let report = report.expect("a sound-only render of a project with sound succeeds");
    let found = probe(&tools, &out);
    std::fs::remove_dir_all(&dir).ok();
    (found, report)
}

#[track_caller]
fn assert_sound_only(found: &Delivered, report: &RenderReport, demuxer: &str, audio: &str) {
    assert!(
        found.is(demuxer),
        "expected {demuxer}, read {}",
        found.container
    );
    assert_eq!(found.video, "", "a sound-only file carries no picture");
    assert_eq!(found.audio, audio, "sound codec");
    assert_eq!(report.frames, 0, "not one frame was drawn");
    assert_eq!(report.resolution, None);
    assert!(
        (report.seconds() - 1.0).abs() < 1e-6,
        "{}",
        report.seconds()
    );
    assert!(
        report.delivered.is_some(),
        "the file is read back and measured"
    );
}

#[test]
fn an_mp3_is_the_soundtrack_alone() {
    let (found, report) = heard("mp3", Container::Mp3);
    assert_sound_only(&found, &report, "mp3", "mp3");
}

#[test]
fn a_wav_is_uncompressed_sound_alone() {
    let (found, report) = heard("wav", Container::Wav);
    assert_sound_only(&found, &report, "wav", "pcm_s16le");
}

#[test]
fn an_m4a_is_aac_alone() {
    let (found, report) = heard("m4a", Container::M4a);
    assert_sound_only(&found, &report, "m4a", "aac");
}

/// The soundtrack a sound-only file carries is the video's, sample for sample:
/// both of these are PCM, so any difference is a difference in the mix.
#[test]
fn the_mix_is_exactly_the_one_the_video_carries() {
    let tools = tools();
    let dir = fixture_dir("same-mix");
    let project = demo(&tools, &dir);
    let (video, avi) = render(&tools, &dir, &project, Container::Avi);
    let (sound, wav) = render(&tools, &dir, &project, Container::Wav);
    video.expect("the video renders");
    sound.expect("the soundtrack renders");
    let (from_video, from_sound) = (samples(&tools, &avi), samples(&tools, &wav));
    std::fs::remove_dir_all(&dir).ok();
    assert!(!from_video.is_empty());
    assert!(from_video == from_sound, "the two soundtracks differ");
}

/// The picture's file is gone, so anything that tried to decode or size it
/// would fail. The soundtrack renders regardless: nothing reached for it.
#[test]
fn a_sound_only_render_never_touches_the_picture() {
    let tools = tools();
    let dir = fixture_dir("no-picture");
    let project = demo(&tools, &dir);
    std::fs::remove_file(dir.join("assets/red.mp4")).expect("the picture existed");
    let (video, _) = render(&tools, &dir, &project, Container::Mp4);
    let (sound, _) = render(&tools, &dir, &project, Container::Wav);
    std::fs::remove_dir_all(&dir).ok();
    assert!(video.is_err(), "the video needs the picture it cannot find");
    sound.expect("the soundtrack never needed it");
}

/// A project of nothing but sound has no picture to decide its length, so its
/// audio does — where a video render of it is refused for having no length.
#[test]
fn a_project_with_no_picture_runs_as_long_as_its_sound() {
    let tools = tools();
    let dir = fixture_dir("sound-project");
    let tone = generate_asset(
        &tools,
        &dir,
        "tone",
        AssetKind::Audio,
        &["-f", "lavfi", "-i", "sine=f=440:d=2", "-ar", "48000"],
    );
    let project = project(
        vec![tone],
        vec![audio_track("a1", vec![clip("c-tone", "tone", 0, 45)])],
    );
    let (video, _) = render(&tools, &dir, &project, Container::Mp4);
    let (sound, _) = render(&tools, &dir, &project, Container::Wav);
    std::fs::remove_dir_all(&dir).ok();
    assert!(matches!(
        video,
        Err(RenderError::Plan(PlanError::NothingToRender))
    ));
    let seconds = sound.expect("a soundtrack of a sound project").seconds();
    assert!(
        (seconds - 1.5).abs() < 1e-6,
        "45 frames at 30 fps, not {seconds}"
    );
}

/// Silence is not a soundtrack: a sound-only file of a timeline nothing on
/// which makes a sound is refused, not written as a plausible empty mp3.
#[test]
fn a_timeline_with_nothing_audible_is_refused() {
    let tools = tools();
    let dir = fixture_dir("nothing-audible");
    let mut project = demo(&tools, &dir);
    project.tracks.retain(|track| track.id.as_str() != "a1");
    let (sound, out) = render(&tools, &dir, &project, Container::Wav);
    let written = out.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        matches!(sound, Err(RenderError::NothingAudible)),
        "{sound:?}"
    );
    assert!(!written, "nothing was left behind");
}

/// The file's audio stream decoded to raw 16-bit samples.
fn samples(tools: &Tools, file: &Path) -> Vec<u8> {
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-map", "0:a", "-f", "s16le", "-"])
        .output()
        .expect("run ffmpeg");
    assert!(output.status.success(), "decoding {file:?}");
    output.stdout
}
