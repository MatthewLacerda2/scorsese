//! Which moments a sheet shows, how big a cell is, and what a label reads.
//!
//! All of it arithmetic, none of it touching a file — so the awkward cases (a
//! file shorter than the window, a range that runs past the end, a source that
//! is taller than it is wide) are decided somewhere they can be checked without
//! decoding anything.

/// The most frames one sheet may hold.
///
/// The compositor's cap, taken rather than restated: two numbers meaning "how
/// many cells" is one of them eventually being wrong.
pub const MAX_FRAMES: usize = scorsese_compositor::sheet::MAX_CELLS;

/// Seconds between frames when no end of the range is given.
pub const STEP_SECONDS: f64 = 5.0;

/// The longest side a cell is scaled to, in pixels.
///
/// A five-cell sheet is then about 1440×540 for ordinary widescreen footage —
/// large enough to see what a shot is of, small enough that looking at footage
/// stays something an assistant does often. Sources smaller than this are never
/// scaled *up*: enlarging pixels adds no information and costs the wire.
pub const CELL_LONGEST_SIDE: u32 = 480;

/// What part of a file to look at, how closely, and whether to rule it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Where to start, in seconds from the beginning of the file.
    pub from_seconds: f64,
    /// Where to stop. Absent means *walk on from `from_seconds`* at
    /// [`STEP_SECONDS`] until the frames run out or the file does.
    pub to_seconds: Option<f64>,
    /// How many frames to take, capped at [`MAX_FRAMES`].
    pub count: usize,
    /// Whether each cell carries a coordinate grid over it.
    ///
    /// Off unless asked for, because the ruler is drawn *on* the picture: it is
    /// what somebody reading a `crop` off the footage wants and the last thing
    /// somebody keeping the sheet does. A cell is one whole source frame, so
    /// the fractions it marks are the source's own — the units `crop` takes.
    pub grid: bool,
}

impl Default for Look {
    /// The whole of what a caller who says nothing gets: five frames from the
    /// start, five seconds apart, and no ruler over them.
    fn default() -> Self {
        Self {
            from_seconds: 0.0,
            to_seconds: None,
            count: MAX_FRAMES,
            grid: false,
        }
    }
}

impl Look {
    /// The moments to decode, in order, none of them past the end of the file.
    ///
    /// The two behaviours are deliberately different, because the two questions
    /// are. **Without an end** this walks the file at a fixed cadence: five
    /// frames, five seconds apart, and a longer file is covered by asking again
    /// from where the last call stopped — 0–20s, then 20–40s. **With an end**
    /// the caller has named a span they want to see, so the frames spread
    /// evenly across it, first on `from` and last on `to`.
    ///
    /// A `duration_seconds` of zero or less means nobody knows how long the
    /// file is, and the answer is then the first moment only: better one frame
    /// than five decodes that fail past the end.
    pub fn moments(&self, duration_seconds: f64) -> Vec<f64> {
        let count = self.count.clamp(1, MAX_FRAMES);
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return vec![self.from_seconds.max(0.0)];
        }
        let last = (duration_seconds - LAST_FRAME_MARGIN).max(0.0);
        let from = tenths_within(self.from_seconds.clamp(0.0, last), last);

        match self.to_seconds {
            Some(to) => spread(from, to.clamp(from, last), count, last),
            None => walk(from, last, count),
        }
    }
}

/// `count` frames spread evenly across a named span, ends included.
fn spread(from: f64, to: f64, count: usize, last: f64) -> Vec<f64> {
    let step = if count > 1 {
        (to - from) / (count - 1) as f64
    } else {
        0.0
    };
    (0..count)
        .map(|index| tenths_within(from + step * index as f64, last))
        .collect()
}

/// How far back from the end the last usable moment sits.
///
/// Seeking to exactly the duration lands past the final frame, and ffmpeg then
/// decodes nothing at all — a blank cell for a file that is perfectly fine.
const LAST_FRAME_MARGIN: f64 = 0.05;

/// Frames at a fixed cadence, stopping where the file does.
///
/// The first one is always taken, even from a file shorter than a single step:
/// a two-second clip has one frame worth looking at, and answering with none
/// would read as *nothing to see* rather than *not much*.
fn walk(from: f64, last: f64, count: usize) -> Vec<f64> {
    let mut moments = Vec::with_capacity(count);
    for index in 0..count {
        let moment = from + STEP_SECONDS * index as f64;
        if index > 0 && moment > last {
            break;
        }
        moments.push(tenths(moment));
    }
    moments
}

/// A moment rounded to a tenth of a second, which is as fine as a label reads
/// and finer than a seek is worth asking for.
fn tenths(seconds: f64) -> f64 {
    (seconds * 10.0).round() / 10.0
}

/// [`tenths`], except never past `last`.
///
/// Rounding happens *after* the clamp to the end of the file, so on its own it
/// can undo one: the last moment of a 12-second file is 11.95, and rounding
/// that to the nearest tenth is 12.0 — a seek past the final frame, which
/// decodes nothing at all. When rounding would overshoot, the tenth below the
/// limit is taken instead.
fn tenths_within(seconds: f64, last: f64) -> f64 {
    let rounded = tenths(seconds);
    if rounded <= last {
        rounded
    } else {
        (last * 10.0).floor() / 10.0
    }
}

/// The size a cell is decoded at, for a source of this shape.
///
/// Aspect is preserved and nothing is padded: every cell of a sheet comes from
/// one file at one scale, so they are all this size and the grid is square
/// without a letterbox anywhere in it. Both sides are made even, because half a
/// pixel of chroma is a scaler warning nobody needs to read.
pub(super) fn cell(width: u32, height: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest == 0 {
        return (2, 2);
    }
    let (width, height) = if longest <= CELL_LONGEST_SIDE {
        (width, height)
    } else {
        let scale = f64::from(CELL_LONGEST_SIDE) / f64::from(longest);
        (
            (f64::from(width) * scale).round() as u32,
            (f64::from(height) * scale).round() as u32,
        )
    };
    (even(width), even(height))
}

/// The nearest even number that is at least two.
fn even(side: u32) -> u32 {
    (side.max(2) / 2) * 2
}

/// A moment as a person reads a timecode: `0:07`, `2:30`, `1:04:09`.
///
/// A tenth is shown only when there is one, which there is only when a caller
/// named a range that does not divide evenly. Whole seconds are the common
/// case and `0:05.0` reads as more precision than anybody asked for.
pub fn label(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let whole = seconds.trunc() as u64;
    let tenth = ((seconds - seconds.trunc()) * 10.0).round() as u64;
    let (whole, tenth) = if tenth >= 10 {
        (whole + 1, 0)
    } else {
        (whole, tenth)
    };

    let mut clock = if whole >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            whole / 3600,
            (whole / 60) % 60,
            whole % 60
        )
    } else {
        format!("{}:{:02}", whole / 60, whole % 60)
    };
    if tenth != 0 {
        clock.push_str(&format!(".{tenth}"));
    }
    clock
}
