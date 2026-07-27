//! The media these tests render, and the clips that place it.

use std::path::Path;

use scorsese_core::{
    Asset, AssetKind, Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath,
};
use scorsese_render::Tools;

use crate::common::ffmpeg::generate_asset;

/// What the fixtures are generated and analysed at.
pub(crate) const RATE: u32 = 48_000;

/// How loud the tone recorded on a video fixture is.
pub(crate) const SHOT_LEVEL: f64 = 0.4;

/// A tone at `hz`, `seconds` long, peaking at `amplitude`, in the project's
/// `assets/`.
///
/// A sine rather than noise or speech: its loudness is the same everywhere, so
/// a window measured anywhere inside it should read the same, and a window that
/// does not is the pipeline's doing rather than the fixture's.
///
/// Written as an explicit expression rather than lavfi's `sine` source, which
/// generates at an eighth of full scale — a default that is nobody's business
/// but ffmpeg's, and one a fixture should not silently inherit. Here the level
/// is a number in the test.
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
        "aevalsrc=exprs={amplitude}*sin(2*PI*{hz}*t)\
         :duration={seconds}:sample_rate={RATE}{filter}"
    );
    generate_asset(
        tools,
        root,
        id,
        AssetKind::Audio,
        &["-f", "lavfi", "-i", &graph],
    )
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
    let graph = format!(
        "aevalsrc=exprs={SHOT_LEVEL}*sin(2*PI*{hz}*t):duration={seconds}:sample_rate={RATE}"
    );
    generate_asset(
        tools,
        root,
        id,
        AssetKind::Video,
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
    )
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
