//! Clicks, dropouts and stepped gain, said so a client can act on a moment.
//!
//! One line per kind that was found, each naming its worst instances by time
//! and channel, as in "3.420 s to 3.940 s left steps 4.1 dB every 16.7 ms
//! (60 Hz)"; and one line in all when nothing was, because a clean capture is
//! the answer to "did the fix land?" and must not read as a report that
//! forgot.

use scorsese_zimmer::level::{Defect, Defects, Tally};

use super::moment;

/// One kind of defect: its name, what it means, how many, and how one is said.
type Kind<'a> = (&'static str, &'static str, &'a Tally, fn(&Defect) -> String);

/// The defects found in one file, as lines to print under its levels.
pub fn defects(found: &Defects) -> Vec<String> {
    if found.is_clean() {
        return vec!["defects  none — no clicks, dropouts or stepped gain".to_owned()];
    }
    let mut lines = Vec::new();
    let kinds: [Kind<'_>; 3] = [
        (
            "clicks",
            "a jump far above every jump around it",
            &found.clicks,
            click,
        ),
        (
            "dropouts",
            "sound gone for a moment",
            &found.dropouts,
            dropout,
        ),
        (
            "stepped gain",
            "the level moves in flat steps, not smoothly",
            &found.zipper,
            staircase,
        ),
    ];
    for (name, meaning, tally, said) in kinds {
        if tally.count == 0 {
            continue;
        }
        let mut line = format!("{name} {} ({meaning}): ", tally.count);
        line.push_str(&tally.worst.iter().map(said).collect::<Vec<_>>().join("; "));
        let more = tally.count.saturating_sub(tally.worst.len());
        if more > 0 {
            line.push_str(&format!("; and {more} more"));
        }
        lines.push(line);
    }
    lines
}

/// How the defects moved between two files: this one's counts, and what the
/// other's were.
pub fn defects_compared(this: &Defects, that: &Defects) -> String {
    let counts = |found: &Defects| [found.clicks.count, found.dropouts.count, found.zipper.count];
    if counts(this) == counts(that) {
        let [clicks, dropouts, steps] = counts(this);
        return format!(
            "defects  unchanged: clicks {clicks}, dropouts {dropouts}, stepped gain {steps}"
        );
    }
    let said: Vec<String> = ["clicks", "dropouts", "stepped gain"]
        .iter()
        .zip(counts(this).into_iter().zip(counts(that)))
        .map(|(name, (now, was))| format!("{name} {now} (was {was})"))
        .collect();
    format!("defects  {}", said.join(", "))
}

fn click(one: &Defect) -> String {
    format!(
        "{} s{}, {:.1} dB over its neighbours",
        moment(one.at_seconds),
        channel(one),
        one.size_db
    )
}

fn dropout(one: &Defect) -> String {
    format!(
        "{} s{} for {:.0} ms, {:.0} dB under the sound either side",
        moment(one.at_seconds),
        channel(one),
        (one.to_seconds - one.from_seconds) * 1000.0,
        one.size_db
    )
}

fn staircase(one: &Defect) -> String {
    let every = one.period_seconds.unwrap_or_default();
    let rate = if every > 0.0 { 1.0 / every } else { 0.0 };
    format!(
        "{} s to {} s{} steps {:.1} dB every {:.1} ms ({rate:.0} Hz)",
        moment(one.from_seconds),
        moment(one.to_seconds),
        channel(one),
        one.size_db,
        every * 1000.0
    )
}

/// Which channel, said after the time; nothing when it is every channel.
fn channel(one: &Defect) -> String {
    match one.channel {
        None => String::new(),
        Some(0) => " left".to_owned(),
        Some(1) => " right".to_owned(),
        Some(n) => format!(" channel {}", n + 1),
    }
}

#[cfg(test)]
mod tests {
    use scorsese_zimmer::level::{Defect, Defects, Tally};

    use super::{defects, defects_compared};

    fn one(channel: Option<usize>, from: f64, to: f64, size_db: f64) -> Defect {
        Defect {
            channel,
            at_seconds: from,
            from_seconds: from,
            to_seconds: to,
            size_db,
            period_seconds: None,
        }
    }

    #[test]
    fn a_clean_file_says_so_in_one_line() {
        assert_eq!(
            defects(&Defects::default()),
            ["defects  none — no clicks, dropouts or stepped gain"]
        );
    }

    #[test]
    fn each_kind_names_its_worst_by_time_and_channel() {
        let found = Defects {
            clicks: Tally {
                count: 7,
                worst: vec![one(Some(1), 1.25, 1.25, 24.84)],
            },
            dropouts: Tally {
                count: 1,
                worst: vec![one(None, 1.0, 1.02, 61.0)],
            },
            zipper: Tally {
                count: 1,
                worst: vec![Defect {
                    period_seconds: Some(1.0 / 60.0),
                    ..one(Some(0), 3.42, 3.94, 4.1)
                }],
            },
        };
        assert_eq!(
            defects(&found),
            [
                "clicks 7 (a jump far above every jump around it): 1.250 s right, \
                 24.8 dB over its neighbours; and 6 more",
                "dropouts 1 (sound gone for a moment): 1.000 s for 20 ms, 61 dB under \
                 the sound either side",
                "stepped gain 1 (the level moves in flat steps, not smoothly): 3.420 s \
                 to 3.940 s left steps 4.1 dB every 16.7 ms (60 Hz)",
            ]
        );
    }

    #[test]
    fn a_comparison_says_what_each_count_was() {
        let mut fixed = Defects::default();
        fixed.clicks.count = 2;
        let mut broken = Defects::default();
        broken.clicks.count = 2;
        broken.zipper.count = 3;
        assert_eq!(
            defects_compared(&fixed, &broken),
            "defects  clicks 2 (was 2), dropouts 0 (was 0), stepped gain 0 (was 3)"
        );
        assert_eq!(
            defects_compared(&fixed, &fixed),
            "defects  unchanged: clicks 2, dropouts 0, stepped gain 0"
        );
    }
}
