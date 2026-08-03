//! A line per song, and the set counted under it.
//!
//! The same shape as the bake report next door — a headline, rows indented
//! beneath it, columns that line up so a reader can scan **down** one of them —
//! because that is the form already familiar from `synth_bake` and
//! `audio_level`, and a report that is read alongside those should not be read
//! differently.
//!
//! **It counts and stops.** Nothing here scores, grades, ranks or advises; see
//! [`scorsese_zimmer::survey`] for why a diversity number is the one metric
//! this must never grow.

use scorsese_zimmer::survey::{Register, Rollup, SongSurvey, Survey, TrackSurvey};

/// The whole report, as lines to print.
///
/// **Empty for a project of fewer than two songs**, which is the rule the
/// section and track tables already hold: one row under a summary of that one
/// row is the same sentence twice. A survey is a report *across a set*, and one
/// song is not one.
///
/// Unlike [`super::sections`], the rows carry their own indentation: this
/// report is two levels deep — tracks under a song, counts under the set — and
/// a caller re-deriving that nesting from a flat list would be reconstructing
/// something this module already knows.
pub fn survey(surveyed: &Survey) -> Vec<String> {
    if surveyed.songs.len() < 2 {
        return Vec::new();
    }
    let name = width(surveyed.songs.iter().map(|song| song.name.len()));
    let bpm = width(surveyed.songs.iter().map(|song| tempo(song.bpm).len()));
    let columns = Columns {
        name: width(tracks(surveyed).map(|(track, _)| track.name.len())),
        source: width(tracks(surveyed).map(|(track, _)| kind(track).len())),
        rate: width(tracks(surveyed).map(|(track, at)| density(track, at).len())),
        median: width(tracks(surveyed).map(|(track, _)| median(track).len())),
    };

    let mut lines = Vec::new();
    for song in &surveyed.songs {
        lines.push(headline(song, name, bpm));
        // A song of one track gets no rows, for the same reason a project of
        // one song gets no report: its only row is the line above it.
        if song.tracks.len() < 2 {
            continue;
        }
        for entry in &song.tracks {
            lines.push(format!("  {}", row(entry, song.seconds, &columns)));
        }
    }
    lines.extend(counted(&surveyed.rollup()));
    lines
}

/// How wide each track column is, measured across the whole report.
///
/// A struct rather than four more arguments: they travel together and are all
/// the same type, which is an argument order waiting to be got wrong silently.
struct Columns {
    name: usize,
    source: usize,
    rate: usize,
    median: usize,
}

/// One song: what it plays at, how far it reaches, and what it is made of.
fn headline(song: &SongSurvey, name: usize, bpm: usize) -> String {
    let mut said = format!(
        "{:<name$}  {:>bpm$} bpm",
        song.name,
        tempo(song.bpm),
        name = name,
        bpm = bpm
    );
    if let Some(register) = song.register {
        said.push_str(&format!("  {register}"));
    }
    if !song.pitches.is_empty() {
        said.push_str(&format!("  {}", song.pitches));
        if let Some(root) = song.pitches.diatonic_root() {
            said.push_str(&format!(" (diatonic on {root})"));
        }
    }
    said
}

/// One track: which instrument, and what it does with itself.
///
/// The order is deliberate. What an instrument **is** — source, gain, filter —
/// leads, because that is what a reader looks a track up for. What it **does**
/// — held or struck, how often, how high — follows, because that is what three
/// cues sharing one complaint turn out to have in common when their source
/// kinds have nothing in common at all.
///
/// `duty` sits beside `gain` rather than with the rest of the second half, and
/// it is printed at all so the rollup's "loudest" is checkable from the rows.
/// That count ranks by `gain × duty`, so without the second number a reader
/// sees a hat written at `0.60` losing to a pad written at `0.50` and has
/// nothing to reconcile it against — which reads as a bug in the report rather
/// than as the fix for one.
fn row(track: &TrackSurvey, seconds: f32, columns: &Columns) -> String {
    format!(
        "{:<name$}  {:<source$}  gain {:>4.2}  duty {:>3.0}%  {}  {:>rate$}  {:>median$}  {}",
        track.name,
        kind(track),
        track.gain,
        track.duty * 100.0,
        sustain(track),
        density(track, seconds),
        median(track),
        cutoff(track),
        name = columns.name,
        source = columns.source,
        rate = columns.rate,
        median = columns.median
    )
}

/// Where the amp envelope settles once the attack and decay are done.
///
/// **The envelope's sustain, not the source's.** A `karplus` string damps by
/// itself and an `fm2` modulator decays by itself, so this can read `0.40` for
/// a sound that dies away regardless. It states what the document states, the
/// same way `gain` does.
fn sustain(track: &TrackSurvey) -> String {
    track.sustain.map_or_else(
        || "sustain    ?".to_owned(),
        |at| format!("sustain {at:>4.2}"),
    )
}

/// How often the track plays, over the length of one pass of the song.
///
/// A track that plays nothing says so in a word. `0.0/s` is arithmetically the
/// same and reads as a measurement of something, when what happened is that
/// this instrument is not in this piece.
fn density(track: &TrackSurvey, seconds: f32) -> String {
    match track.density(seconds) {
        Some(rate) if track.notes > 0 => format!("{rate:.1}/s"),
        _ => "silent".to_owned(),
    }
}

/// Where the track sits, as the name of the middle note it plays.
///
/// [`Register`] is what names a pitch here, and its `Display` already answers
/// the one-note case as the note itself — a median is exactly that case, so
/// this is the vocabulary the register column uses rather than a second one.
fn median(track: &TrackSurvey) -> String {
    track
        .median
        .map_or_else(|| "-".to_owned(), |midi| Register::at(midi).to_string())
}

/// The set, counted — the half no per-song row can add up on its own.
fn counted(rollup: &Rollup) -> Vec<String> {
    let label = width(
        rollup
            .sources
            .iter()
            .map(|count| count.source.len())
            .chain([REGISTER.len()]),
    );
    let mut lines = vec![format!("{} songs", rollup.songs)];
    for count in &rollup.sources {
        lines.push(format!(
            "  {:<label$}  in {}, loudest in {}",
            count.source,
            count.songs,
            count.loudest,
            label = label
        ));
    }
    if let Some((low, high)) = rollup.tempo {
        let spread = if low == high {
            tempo(low)
        } else {
            format!("{}-{}", tempo(low), tempo(high))
        };
        lines.push(format!(
            "  {:<label$}  {spread} bpm",
            "tempo",
            label = label
        ));
    }
    if let Some(register) = rollup.register {
        lines.push(format!("  {REGISTER:<label$}  {register}", label = label));
    }
    lines
}

/// The label the widest rollup row carries, named once so the column width and
/// the row that sets it cannot drift apart.
const REGISTER: &str = "register";

/// A tempo as a reader writes one: `96`, and `96.5` only when it is.
fn tempo(bpm: f32) -> String {
    format!("{bpm}")
}

/// What plays a track, or that its instrument could not be read — which is a
/// finding rather than a blank, the same way a silent layer is.
fn kind(track: &TrackSurvey) -> &'static str {
    track.source.unwrap_or("unresolved")
}

/// Where a track's filter starts, or that it has none.
fn cutoff(track: &TrackSurvey) -> String {
    track
        .cutoff
        .map_or_else(|| "no filter".to_owned(), |hz| format!("cutoff {hz:.0} Hz"))
}

/// The widest of some lengths, and zero when there are none.
fn width(lengths: impl Iterator<Item = usize>) -> usize {
    lengths.max().unwrap_or(0)
}

/// Every track of every song, each with the length of the song it is in, for
/// the columns that are sized across the whole report rather than per song — a
/// column a reader can scan is one that does not move between one song and the
/// next.
fn tracks(surveyed: &Survey) -> impl Iterator<Item = (&TrackSurvey, f32)> {
    surveyed
        .songs
        .iter()
        .filter(|song| song.tracks.len() > 1)
        .flat_map(|song| song.tracks.iter().map(|track| (track, song.seconds)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_zimmer::survey::{PitchClasses, TrackSurvey};

    /// Eight notes over the four seconds `song` is long, so every fixture track
    /// plays at 2.0/s unless a test says otherwise.
    fn track(name: &str, source: &'static str, gain: f32, cutoff: Option<f32>) -> TrackSurvey {
        TrackSurvey {
            name: name.to_owned(),
            source: Some(source),
            gain,
            cutoff,
            sustain: Some(0.0),
            notes: 8,
            median: Some(71.0),
            duty: 0.5,
        }
    }

    fn song(name: &str, bpm: f32, tracks: Vec<TrackSurvey>) -> SongSurvey {
        let mut pitches = PitchClasses::default();
        for midi in [60.0, 62.0, 64.0] {
            pitches.add(midi);
        }
        SongSurvey {
            name: name.to_owned(),
            bpm,
            tracks,
            seconds: 4.0,
            register: Some(Register::at(40.0).with(81.0)),
            pitches,
        }
    }

    fn two() -> Survey {
        Survey {
            songs: vec![
                song(
                    "01-km",
                    96.0,
                    vec![
                        track("arp", "karplus", 0.85, Some(4600.0)),
                        track("pad", "osc_stack", 0.4, None),
                    ],
                ),
                song(
                    "02-calado",
                    132.0,
                    vec![
                        track("arp", "karplus", 0.76, Some(4400.0)),
                        track("bell", "fm2", 0.5, None),
                    ],
                ),
            ],
        }
    }

    /// The report the whole thing exists for: two songs, their instruments,
    /// and the count that says one kind is the loudest in both.
    #[test]
    fn the_report_is_a_line_per_song_and_the_set_counted_under_it() {
        let lines = survey(&two());
        assert!(lines[0].starts_with("01-km"), "{}", lines[0]);
        assert!(lines[0].contains("96 bpm"), "{}", lines[0]);
        assert!(lines[0].contains("E2-A5"), "{}", lines[0]);
        assert!(lines[0].contains("C D E"), "{}", lines[0]);
        assert!(lines[1].contains("arp"), "{}", lines[1]);
        assert!(lines[1].contains("karplus"), "{}", lines[1]);
        assert!(lines[1].contains("gain 0.85"), "{}", lines[1]);
        assert!(lines[1].contains("cutoff 4600 Hz"), "{}", lines[1]);
        assert!(lines[2].contains("no filter"), "{}", lines[2]);

        let rollup = lines.join("\n");
        assert!(rollup.contains("2 songs"), "{rollup}");
        assert!(rollup.contains("karplus    in 2, loudest in 2"), "{rollup}");
        assert!(rollup.contains("fm2        in 1, loudest in 0"), "{rollup}");
        assert!(rollup.contains("96-132 bpm"), "{rollup}");
    }

    /// The columns are sized across the whole report, so a reader can scan one
    /// of them down past the song it started in.
    #[test]
    fn the_track_columns_line_up_between_one_song_and_the_next() {
        let lines = survey(&two());
        let at = |row: &String| row.find("gain").expect("every row has a gain");
        assert_eq!(at(&lines[1]), at(&lines[4]));
    }

    /// The half the report was missing: whether a track is held or struck, how
    /// often it plays, and where it sits. Three cues that read alike on the
    /// left of the row are told apart here.
    #[test]
    fn a_row_says_what_the_track_does_and_not_only_what_it_is() {
        let lines = survey(&two());
        assert!(lines[1].contains("sustain 0.00"), "{}", lines[1]);
        assert!(lines[1].contains("2.0/s"), "{}", lines[1]);
        assert!(lines[1].contains("B4"), "{}", lines[1]);
    }

    /// The rollup ranks by `gain × duty`, so the duty has to be on the row or
    /// the count below it looks like it contradicts the gains above it.
    #[test]
    fn a_row_shows_the_duty_the_rollup_ranks_by() {
        let mut cue = two();
        cue.songs[0].tracks[0] = TrackSurvey {
            duty: 0.2,
            ..cue.songs[0].tracks[0].clone()
        };
        let lines = survey(&cue);
        assert!(lines[1].contains("gain 0.85  duty  20%"), "{}", lines[1]);
        assert!(lines[2].contains("duty  50%"), "{}", lines[2]);
    }

    /// A track nobody wrote a note for is *silent*, not `0.0/s`, and has no
    /// middle note to name. Both are findings rather than measurements of zero.
    #[test]
    fn a_track_that_never_plays_says_so_in_a_word() {
        let mut quiet = two();
        quiet.songs[0].tracks[1] = TrackSurvey {
            notes: 0,
            median: None,
            sustain: None,
            ..quiet.songs[0].tracks[1].clone()
        };
        let lines = survey(&quiet);
        assert!(lines[2].contains("silent"), "{}", lines[2]);
        assert!(!lines[2].contains("0.0/s"), "{}", lines[2]);
        assert!(lines[2].contains("sustain    ?"), "{}", lines[2]);
    }

    /// One song is not a set, so there is no report at all — the rule the
    /// section rows and the track rows already follow.
    #[test]
    fn a_project_of_one_song_says_nothing() {
        let one = Survey {
            songs: vec![song(
                "only",
                120.0,
                vec![track("arp", "karplus", 0.9, None)],
            )],
        };
        assert!(survey(&one).is_empty());
        assert!(survey(&Survey::default()).is_empty());
    }
}
