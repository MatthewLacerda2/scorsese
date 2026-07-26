//! The media these tests render, and the clips that place it.

use std::path::Path;

use scorsese_core::{
    Asset, AssetKind, Clip, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath,
};
use scorsese_render::Tools;

use crate::common::ffmpeg::generate_asset;

/// What the fixtures are generated and analysed at.
pub const RATE: u32 = 48_000;

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
pub fn tone_asset(
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

/// Writes a volume keyframe track onto a clip, in one call, since every audio
/// test that animates anything animates this.
pub fn with_volume(mut clip: Clip, points: &[(u64, f64)]) -> Clip {
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
