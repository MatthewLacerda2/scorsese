//! How much the master limiter took off, said where it took it (#1795).
//!
//! The limiter holds every bake under −1 dBTP, silently, and every other
//! figure in the report is measured after it — so a mix written too hot comes
//! back not clipping, levelled flat, and with nothing saying why. These pin
//! the figure that does: on the summary, on the row it flattened, and over a
//! window exactly as over the whole.

use scorsese_zimmer::patch::{Source, Wave};
use scorsese_zimmer::song::{InlineOnly, PatchRef, Pattern, Song, Track};
use scorsese_zimmer::{Excerpt, NoteOpts, Span, Window, bake_excerpt, bake_note, bake_song};

use crate::common::songs::{note, played, song};
use crate::common::{osc, saw_patch};

/// A sine held at full level for as long as it is written — so a hot track is
/// limited all the way through its section, not for one transient, by an
/// amount its peak says in advance.
fn held() -> scorsese_zimmer::Patch {
    scorsese_zimmer::Patch {
        source: Source::OscStack {
            oscs: vec![osc(Wave::Sine, 0.0, 0)],
        },
        ..saw_patch()
    }
}

fn track(name: &str, gain: f32) -> Track {
    Track {
        name: name.to_owned(),
        patch: PatchRef::Inline(Box::new(held())),
        gain,
        ..song().tracks[0].clone()
    }
}

/// Two four-beat sections at 120 bpm — two seconds each: `loud`, a track at
/// `hot` gain held throughout, and `quiet`, a track far under the ceiling.
fn loud_then_quiet(hot: f32) -> Song {
    let mut song = song();
    song.tracks = vec![track("hot", hot), track("soft", 0.05)];
    song.patterns.clear();
    song.patterns.insert(
        "loud".to_owned(),
        Pattern {
            beats: 4.0,
            notes: played(vec![note("hot", "C4", 0.0, 4.0)]),
        },
    );
    song.patterns.insert(
        "quiet".to_owned(),
        Pattern {
            beats: 4.0,
            notes: played(vec![note("soft", "C4", 0.0, 4.0)]),
        },
    );
    song.arrangement = vec!["loud".into(), "quiet".into()];
    song
}

fn limited(span: &scorsese_zimmer::level::Span) -> f64 {
    span.limited_db
        .expect("a bake hands its limiter's gain to the report")
}

/// A sine peaking at twice full scale is taken down by its 6 dB of overshoot
/// and the ceiling's 1 dB of headroom — and says so on its own row and not
/// on the quiet one beside it.
#[test]
fn a_limited_section_says_so_on_its_own_row() {
    let bake = bake_song(&loud_then_quiet(2.0), &InlineOnly).expect("it renders");
    let rows = &bake.profile.sections;
    assert_eq!(rows[0].label.as_deref(), Some("loud"));
    let loud = limited(&rows[0]);
    assert!((loud - 7.0).abs() < 0.2, "the loud row took off {loud} dB");
    // The quiet row starts while the gain is still recovering from 7 dB over
    // the release, which averages to a few tenths over two seconds: under
    // the half a decibel a row has to reach before it is said.
    let quiet = limited(&rows[1]);
    assert!(quiet < 0.5, "the quiet row took off {quiet} dB");
    let whole = limited(&bake.profile.whole);
    assert!(
        whole > quiet && whole < loud,
        "the whole is between: {whole}"
    );
}

/// A mix that never reaches the ceiling is never limited, and says so as a
/// zero rather than as a missing figure: the bake had a gain track, it was
/// just 1.0 throughout.
#[test]
fn a_mix_that_never_reaches_the_ceiling_is_limited_by_nothing() {
    let bake = bake_song(&loud_then_quiet(0.1), &InlineOnly).expect("it renders");
    assert_eq!(bake.profile.whole.limited_db, Some(0.0));
    assert!(
        bake.profile
            .sections
            .iter()
            .all(|row| row.limited_db == Some(0.0))
    );
    // The track rows are measured before the limiter, so they never carry it.
    assert!(
        bake.tracks
            .iter()
            .all(|track| track.level.limited_db.is_none())
    );
}

/// An excerpt runs the same limiter over the same piece, so what it says over
/// a window is what the whole bake said there — the promise every other
/// figure of a partial bake keeps.
#[test]
fn an_excerpts_figure_is_the_whole_bakes_over_the_same_window() {
    let song = loud_then_quiet(2.0);
    let whole = bake_song(&song, &InlineOnly).expect("it renders");
    let window = Excerpt::of(Window::seconds(
        Span::new(0.0, Some(2.0)).expect("a legal span"),
    ));
    let part = bake_excerpt(&song, &InlineOnly, &window).expect("it renders");
    let said = limited(&part.profile.whole);
    let there = limited(&whole.profile.sections[0]);
    assert!((said - there).abs() < 1e-9, "{said} against {there}");
}

/// A one-shot is limited too, and gets the summary figure: it has no rows.
#[test]
fn a_one_shot_says_how_much_it_was_limited() {
    let bake = bake_note(&held(), 60.0, &NoteOpts::default()).expect("it renders");
    assert!(bake.profile.sections.is_empty());
    assert!(bake.profile.whole.limited_db.is_some());
}

/// A gain track is averaged in decibels, so a factor of zero would be an
/// infinity and a `NaN` would poison the row: both count as untouched, and a
/// gain the run does not reach counts as 1.0.
#[test]
fn a_gain_that_is_not_a_positive_number_counts_as_untouched() {
    let mut profiler = scorsese_zimmer::level::Profiler::new(1, 1_000);
    let half = 10f32.powf(-6.0 / 20.0);
    profiler.feed_limited(&[0.5; 4], &[half, 0.0, f32::NAN]);
    let taken = profiler
        .finish()
        .whole
        .limited_db
        .expect("a gain track was fed");
    assert!(
        (taken - 1.5).abs() < 1e-6,
        "6 dB over one frame of four: {taken}"
    );
}
