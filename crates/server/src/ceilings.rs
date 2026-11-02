//! How much local work one request may cause on the shared machine (#1634):
//! how big and how long a render, a preview, a page capture or a still is,
//! and how long an uploaded file plays.
//!
//! Bytes and money are bounded elsewhere (`library::upload::MAX_UPLOAD_BYTES`,
//! `config::PerUser`); these bound the **decode and draw work** a request
//! asks for, which bytes do not: a small file can play for hours, and a
//! one-line request can ask for an 8K hour. Each is a default good enough
//! never to touch, and each refusal says what the limit is and what to do.
//! Web only: the CLI, the MCP server and the desktop app run on the user's
//! own machine and keep none of these.

use std::time::Duration;

use scorsese_core::{Fps, Frames, Project};
use scorsese_render::{Lengths, Resolution};

use crate::renders::Settings;

/// The longest side of a picture: 4K's, the top of what Filmora 9 exports and
/// of what any `scorsese_core::style::Platform` takes.
pub const LONGEST_SIDE: u32 = 3840;

/// The shortest side of a picture, so 4K either way up.
pub const SHORTEST_SIDE: u32 = 2160;

/// The fastest rate a render or a capture plays at.
pub const FASTEST: u32 = 60;

/// The longest a render runs: every kind of video in `docs/styles.md`, with
/// room to spare.
pub const LONGEST_RENDER: Duration = Duration::from_secs(60 * 60);

/// The longest an uploaded video or sound plays: a long screen recording or a
/// whole podcast episode, the longest thing an approachable edit starts from.
pub const LONGEST_FILE: Duration = Duration::from_secs(3 * 60 * 60);

/// Whether a picture `width` × `height` is within [`LONGEST_SIDE`] and
/// [`SHORTEST_SIDE`], whichever way up it is.
pub fn picture(width: u32, height: u32) -> Result<(), String> {
    let (long, short) = (width.max(height), width.min(height));
    if long <= LONGEST_SIDE && short <= SHORTEST_SIDE {
        return Ok(());
    }
    Err(format!(
        "pictures here are at most {LONGEST_SIDE}x{SHORTEST_SIDE} (or {SHORTEST_SIDE}x\
         {LONGEST_SIDE} upright); this one is {width}x{height}: ask for a smaller resolution"
    ))
}

/// Whether `fps` is at most [`FASTEST`].
pub fn rate(fps: Fps) -> Result<(), String> {
    if u64::from(fps.num()) <= u64::from(FASTEST) * u64::from(fps.den()) {
        return Ok(());
    }
    Err(format!(
        "renders here play at most {FASTEST} frames a second; this one asks for {}: ask for \
         {FASTEST} or fewer",
        trimmed(fps.as_f64())
    ))
}

/// Whether `frames` on an `fps` grid last at most [`LONGEST_RENDER`].
pub fn length(frames: u64, fps: Fps) -> Result<(), String> {
    let most = u128::from(LONGEST_RENDER.as_secs()) * u128::from(fps.num());
    if u128::from(frames) * u128::from(fps.den()) <= most {
        return Ok(());
    }
    Err(format!(
        "renders here are at most {} minutes long; this one is {}: render a range, or trim \
         the timeline",
        LONGEST_RENDER.as_secs() / 60,
        spoken(fps.seconds(Frames(frames)))
    ))
}

/// Whether a render of `project` in `settings` is within every ceiling: its
/// picture and its rate (the project's own when none is asked), when it has
/// a picture, and its length (the range asked for, else the whole timeline).
pub fn render(project: &Project, settings: &Settings) -> Result<(), String> {
    if let Some(size) = &settings.resolution {
        let size: Resolution = size.parse().map_err(|e| format!("resolution: {e}"))?;
        picture(size.width(), size.height())?;
        match &settings.fps {
            Some(fps) => rate(fps.parse::<Fps>().map_err(|e| format!("fps: {e}"))?)?,
            None => rate(project.timeline_fps).map_err(|_| {
                format!(
                    "renders here play at most {FASTEST} frames a second, and this project's \
                     timeline runs at {}: ask for an fps of {FASTEST} or fewer",
                    trimmed(project.timeline_fps.as_f64())
                )
            })?,
        }
    }
    length(rendered(project, settings)?, project.timeline_fps)
}

/// The furthest a page's own clock may run in one capture, the frames run
/// through without drawing included: as long as the longest file plays, so
/// a page clip sped up or started late in a render inside [`LONGEST_RENDER`]
/// is still captured.
pub const LONGEST_PAGE: Duration = LONGEST_FILE;

/// Whether a capture of a page at `fps`, its clock running to frame `end`,
/// is within [`LONGEST_PAGE`].
pub fn page_clock(end: u64, fps: Fps) -> Result<(), String> {
    let most = u128::from(LONGEST_PAGE.as_secs()) * u128::from(fps.num());
    if u128::from(end) * u128::from(fps.den()) <= most {
        return Ok(());
    }
    Err(format!(
        "a page here runs at most {} hours of its own clock; this one runs {}: start its clip \
         nearer the page's beginning, or play it slower",
        LONGEST_PAGE.as_secs() / 3600,
        spoken(fps.seconds(Frames(end)))
    ))
}

/// How many of `project`'s timeline frames a render in `settings` makes: the
/// range asked for, cut to the timeline, else the whole timeline.
pub fn rendered(project: &Project, settings: &Settings) -> Result<u64, String> {
    let range = settings.range()?;
    let end = range
        .end()
        .map_or(project.length(), |end| end.min(project.length()));
    Ok(end.get().saturating_sub(range.start().get()))
}

/// How far a stream may run past its file's container before the file is
/// taken to disagree with itself: a minute, or a tenth of the file, whichever
/// is more — what a variable-rate stream or a trailing subtitle drifts by.
const DRIFT: f64 = 60.0;

/// Whether a file whose probe says `lengths` may be kept: its longest word
/// for how long it plays at most [`LONGEST_FILE`]. A video or sound that
/// gives no length, or whose stream runs far past its container, is refused
/// rather than trusted — the decode is what costs, and it runs for as long as
/// the longest stream does.
pub fn playing(lengths: &Lengths) -> Result<(), String> {
    let known = |seconds: &&f64| seconds.is_finite() && **seconds >= 0.0;
    let longest_stream = lengths
        .streams
        .iter()
        .filter(known)
        .copied()
        .reduce(f64::max);
    let container = lengths.container.filter(|seconds| known(&seconds));
    let Some(plays) = container.or(longest_stream) else {
        return Err(
            "the file does not say how long it plays, so it cannot be kept here: \
                    export it again from the program that made it"
                .to_owned(),
        );
    };
    if let Some(stream) = longest_stream
        && stream > plays + DRIFT.max(plays / 10.0)
    {
        return Err(format!(
            "the file does not agree with itself: it says it plays {}, but one of its \
             streams runs {}; export it again from the program that made it",
            spoken(plays),
            spoken(stream)
        ));
    }
    let plays = longest_stream.map_or(plays, |stream| stream.max(plays));
    if plays <= LONGEST_FILE.as_secs_f64() {
        return Ok(());
    }
    Err(format!(
        "files here play for at most {} hours; this one plays {}: cut it before uploading",
        LONGEST_FILE.as_secs() / 3600,
        spoken(plays)
    ))
}

/// `seconds` as a person says it: `5 h 12 min`, `61 min`, `14 s` — rounded up
/// to the unit shown last, so a length just past a ceiling never reads as the
/// ceiling itself.
pub fn spoken(seconds: f64) -> String {
    let whole = seconds.max(0.0).ceil() as u64;
    let (hours, minutes, rest) = (whole / 3600, whole % 3600 / 60, whole % 60);
    match (hours, minutes, rest) {
        (0, 0, s) => format!("{s} s"),
        (0, m, 0) => format!("{m} min"),
        (0, m, s) => format!("{m} min {s} s"),
        (h, 0, 0) => format!("{h} h"),
        (h, m, 0) => format!("{h} h {m} min"),
        (h, m, s) => format!("{h} h {m} min {s} s"),
    }
}

/// A rate as a person writes it: `61`, `59.94`.
fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_k_either_way_up_is_the_largest_picture() {
        assert_eq!(picture(3840, 2160), Ok(()));
        assert_eq!(picture(2160, 3840), Ok(()));
        assert_eq!(picture(2160, 2160), Ok(()));
        for (width, height) in [(3841, 2160), (2160, 3841), (3000, 2161), (2161, 2161)] {
            let said = picture(width, height).unwrap_err();
            assert!(said.contains(&format!("{width}x{height}")), "{said}");
        }
    }

    #[test]
    fn sixty_frames_a_second_is_the_fastest() {
        assert_eq!(rate(Fps::SIXTY), Ok(()));
        assert_eq!(rate(Fps::new(60000, 1001).unwrap()), Ok(()));
        let said = rate(Fps::new(61, 1).unwrap()).unwrap_err();
        assert!(said.contains("asks for 61:"), "{said}");
        assert!(rate(Fps::new(60001, 1000).unwrap()).is_err());
    }

    #[test]
    fn an_hour_is_the_longest_render_to_the_frame() {
        assert_eq!(length(108_000, Fps::THIRTY), Ok(()));
        assert_eq!(length(216_000, Fps::SIXTY), Ok(()));
        let said = length(108_001, Fps::THIRTY).unwrap_err();
        assert!(said.contains("this one is 1 h 0 min 1 s"), "{said}");
        assert_eq!(length(107_892, Fps::NTSC), Ok(()));
        assert!(length(107_893, Fps::NTSC).is_err());
    }

    fn plays(container: Option<f64>, streams: &[f64]) -> Result<(), String> {
        playing(&Lengths {
            container,
            streams: streams.to_vec(),
        })
    }

    #[test]
    fn three_hours_is_the_longest_file() {
        let three = 3.0 * 3600.0;
        assert_eq!(plays(Some(three), &[three - 0.5, three]), Ok(()));
        assert_eq!(plays(None, &[three]), Ok(()));
        let said = plays(Some(three + 1.0), &[]).unwrap_err();
        assert!(said.contains("plays 3 h 0 min 1 s"), "{said}");
        let said = plays(Some(5.0 * 3600.0 + 12.0 * 60.0), &[]).unwrap_err();
        assert!(said.contains("plays 5 h 12 min:"), "{said}");
        // A stream a little past its container counts at its own length.
        assert!(plays(Some(three), &[three + 30.0]).is_err());
    }

    #[test]
    fn a_file_without_a_length_or_at_odds_with_itself_is_refused() {
        for unknown in [None, Some(f64::NAN), Some(f64::INFINITY), Some(-1.0)] {
            let said = plays(unknown, &[]).unwrap_err();
            assert!(said.contains("does not say"), "{said}");
        }
        let said = plays(Some(10.0), &[10.0, 9000.0]).unwrap_err();
        assert!(said.contains("does not agree"), "{said}");
        assert_eq!(plays(Some(10.0), &[69.0]), Ok(()));
        // The drift allowed is a tenth of a long file, to the second.
        assert_eq!(plays(Some(2000.0), &[2200.0]), Ok(()));
        assert!(plays(Some(2000.0), &[2201.0]).is_err());
    }

    #[test]
    fn a_page_may_run_its_clock_three_hours() {
        assert_eq!(page_clock(3 * 3600 * 60, Fps::SIXTY), Ok(()));
        let said = page_clock(3 * 3600 * 60 + 1, Fps::SIXTY).unwrap_err();
        assert!(said.contains("at most 3 hours of its own clock"), "{said}");
        // On a fractional rate, to the frame: 10,800 s at 30000/1001.
        assert_eq!(page_clock(323_676, Fps::NTSC), Ok(()));
        assert!(page_clock(323_677, Fps::NTSC).is_err());
    }

    #[test]
    fn a_length_is_said_rounded_up_to_its_last_unit() {
        assert_eq!(spoken(0.2), "1 s");
        assert_eq!(spoken(60.0), "1 min");
        assert_eq!(spoken(72.0 * 60.0), "1 h 12 min");
        assert_eq!(spoken(3600.0), "1 h");
        assert_eq!(spoken(61.5), "1 min 2 s");
    }
}
