//! The media these tests render, and the clips that place it.

use std::path::{Path, PathBuf};

use scorsese_core::{
    Asset, AssetKind, Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath,
};
use scorsese_render::Tools;
use scorsese_render::testing::{tone, tone_graph};

use crate::common::ffmpeg::{generate, generate_asset};

/// What the fixtures are generated and analysed at.
pub(crate) use scorsese_render::testing::RATE;

/// How loud the tone recorded on a video fixture is.
pub(crate) const SHOT_LEVEL: f64 = 0.4;

/// A tone at `hz`, `seconds` long, peaking at `amplitude`, in the project's
/// `assets/`.
///
/// A sine rather than noise or speech: its loudness is the same everywhere, so
/// a window measured anywhere inside it should read the same, and a window that
/// does not is the pipeline's doing rather than the fixture's. Full scale is
/// `1.0`, never lavfi `sine`'s eighth ([`tone_graph`] says why).
///
/// `filter` is appended to the graph, which is how a fixture gains a run of
/// silence at the front without a second helper.
pub(crate) fn tone_asset(
    tools: &Tools,
    root: &Path,
    id: &str,
    hz: u32,
    seconds: f64,
    amplitude: f64,
    filter: &str,
) -> Asset {
    let graph = format!(
        "{}{filter}",
        tone_graph(hz, &amplitude.to_string(), seconds)
    );
    generate_asset(
        tools,
        root,
        id,
        AssetKind::Audio,
        &["-f", "lavfi", "-i", &graph],
    )
}

/// A file named `{name}.wav` in `dir`: `seconds` of a 440 Hz sine whose
/// amplitude is whatever `amplitude` evaluates to — a constant, or something
/// that changes partway through. A loose file rather than an asset, for the
/// tests that read a file on disk instead of rendering a project.
pub(crate) fn tone_file(
    tools: &Tools,
    dir: &Path,
    name: &str,
    amplitude: &str,
    seconds: f64,
) -> PathBuf {
    let file = dir.join(format!("{name}.wav"));
    tone(tools, &file, 440, amplitude, seconds);
    file
}

/// A video asset with a tone recorded **on it** — a stand-in for an interview,
/// a talking head, or a screen recording with a click track.
///
/// The picture is black and nobody looks at it. What matters is that the sound
/// is inside the same file as the picture, which is the whole of what makes
/// this different from an audio asset sitting next to one.
///
/// Its tone peaks at [`SHOT_LEVEL`] rather than near full scale, so a second
/// sound over it still sums inside full scale — what these tests measure is the
/// mix, not the clipper.
pub(crate) fn talking_picture(
    tools: &Tools,
    root: &Path,
    id: &str,
    seconds: u32,
    hz: u32,
) -> Asset {
    talking(tools, &root.join("assets").join(name(id)), seconds, hz);
    crate::common::file_asset(id, AssetKind::Video)
}

/// The same file, written where a **generated** shot lands, and described by an
/// asset in the state one is in when it lands there.
///
/// Veo returns video with sound on it, and a shot that has just been generated
/// carries no `media` block — nothing has measured it, and its brief could not
/// have said. Which makes this the fixture for the render finding out for
/// itself, and the shape of the film that #249 rendered silent.
pub(crate) fn generated_talking_shot(
    tools: &Tools,
    root: &Path,
    id: &str,
    seconds: u32,
    hz: u32,
) -> Asset {
    talking(tools, &root.join("generated").join(name(id)), seconds, hz);
    crate::common::generated_asset(id, AssetKind::GeneratedVideo)
}

/// A video asset whose recorded sound starts `offset` seconds **after** its
/// picture: a second of tone at `hz` muxed with `-itsoffset`, so the audio
/// stream's `start_time` is late while the picture's and the container's are
/// zero — the shape of a phone recording or an edited export (#1601).
pub(crate) fn late_talking_picture(
    tools: &Tools,
    root: &Path,
    id: &str,
    seconds: u32,
    hz: u32,
    offset: f64,
) -> Asset {
    let graph = tone_graph(hz, &SHOT_LEVEL.to_string(), 1.0);
    generate(
        tools,
        &root.join("assets").join(name(id)),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:s=32x32:d={seconds}:r=30"),
            "-itsoffset",
            &offset.to_string(),
            "-f",
            "lavfi",
            "-i",
            &graph,
            "-pix_fmt",
            "yuv420p",
        ],
    );
    crate::common::file_asset(id, AssetKind::Video)
}

/// What both of the above are named after their asset id.
fn name(id: &str) -> String {
    format!("{id}.mp4")
}

/// Writes a black picture with a tone recorded on it to `file`.
fn talking(tools: &Tools, file: &Path, seconds: u32, hz: u32) {
    let graph = tone_graph(hz, &SHOT_LEVEL.to_string(), f64::from(seconds));
    generate(
        tools,
        file,
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:s=32x32:d={seconds}:r=30"),
            "-f",
            "lavfi",
            "-i",
            &graph,
            "-shortest",
            "-pix_fmt",
            "yuv420p",
        ],
    );
}

/// Writes a volume keyframe track onto a clip, in one call, since every audio
/// test that animates anything animates this.
pub(crate) fn with_volume(mut clip: Clip, points: &[(u64, f64)]) -> Clip {
    clip.keyframes.push(KeyframeTrack::new(
        PropertyPath::new(scorsese_render::audio::path::VOLUME),
        points
            .iter()
            .map(|&(t, value)| Keyframe {
                t: Frames(t),
                value,
                easing: Easing::Linear,
            })
            .collect(),
    ));
    clip
}
