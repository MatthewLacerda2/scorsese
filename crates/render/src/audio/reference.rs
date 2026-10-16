//! What five real records measure, as numbers a bake can be held against
//! (#1125).
//!
//! `docs/references.md` is where these were measured and argued over; this is
//! the same table in a form the bake report can read, so the comparison is in
//! front of the agent at the moment it decides rather than in a page it has to
//! remember to open. A test holds the page's tables to these figures, so the
//! two cannot drift.
//!
//! **A floor, never a gate.** Nothing here can fail a bake: a score that
//! matches every range can still sound like a home keyboard, and the page says
//! why at length.

/// A spread of measurements, lowest first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    /// The lowest value the record's body reached.
    pub low: f64,
    /// The highest.
    pub high: f64,
}

impl Range {
    const fn new(low: f64, high: f64) -> Self {
        Self { low, high }
    }

    /// Whether `value` sits inside it, ends included.
    pub fn holds(&self, value: f64) -> bool {
        (self.low..=self.high).contains(&value)
    }
}

/// One reference record's body: the 8-second windows after its intro and
/// before its fade, measured with `scorsese level`.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    /// What a caller names it by: `billie-jean`.
    pub id: &'static str,
    /// What the page's tables call it, exactly: `Billie Jean`.
    pub row: &'static str,
    /// Body mean, in dBFS. The record's master runs over full scale, so this
    /// is louder than a bake held at −1 dBTP can be.
    pub mean: Range,
    /// Body crest (peak minus mean), in dB.
    pub crest: Range,
    /// Body low, mid and high shares, in percent.
    pub low: Range,
    /// See [`Self::low`].
    pub mid: Range,
    /// See [`Self::low`].
    pub high: Range,
    /// Body stereo correlation. `None` where the file is mono, so its width
    /// says nothing about the record.
    pub correlation: Option<Range>,
    /// The crest to aim a bake at, where the page names one.
    pub aim_crest: Option<f64>,
    /// How far the record's first 8 seconds sit under its body's mean, in dB.
    /// Negative is an opening louder than the body.
    pub opening_under_db: f64,
}

impl Reference {
    /// The body mean a bake reaches at [`Self::aim_crest`]: zimmer's limiter
    /// holds the true peak at −1 dBTP, and crest is peak minus mean.
    pub fn reachable_mean(&self) -> Option<f64> {
        self.aim_crest.map(|crest| CEILING_DBTP - crest)
    }
}

/// Where zimmer's limiter holds a bake's true peak.
const CEILING_DBTP: f64 = -1.0;

/// Every reference, in the page's order.
pub const REFERENCES: &[Reference] = &[
    Reference {
        id: "billie-jean",
        row: "Billie Jean",
        mean: Range::new(-16.5, -12.5),
        crest: Range::new(14.0, 18.5),
        low: Range::new(56.0, 86.0),
        mid: Range::new(11.0, 40.0),
        high: Range::new(2.0, 5.0),
        correlation: Some(Range::new(0.6, 0.97)),
        aim_crest: Some(15.5),
        opening_under_db: 2.0,
    },
    Reference {
        id: "fuego-drops",
        row: "Fuego, drops",
        mean: Range::new(-10.0, -8.0),
        crest: Range::new(9.5, 11.5),
        low: Range::new(74.0, 85.0),
        mid: Range::new(14.0, 25.0),
        high: Range::new(1.0, 2.0),
        correlation: Some(Range::new(0.75, 0.95)),
        aim_crest: Some(10.0),
        opening_under_db: 18.0,
    },
    Reference {
        id: "fuego-breaks",
        row: "Fuego, breaks",
        mean: Range::new(-19.0, -15.5),
        crest: Range::new(13.0, 17.5),
        low: Range::new(9.0, 39.0),
        mid: Range::new(61.0, 89.0),
        high: Range::new(0.0, 5.0),
        correlation: Some(Range::new(0.2, 0.5)),
        aim_crest: None,
        opening_under_db: 10.0,
    },
    Reference {
        id: "techno-syndrome",
        row: "Techno Syndrome",
        mean: Range::new(-7.5, -5.5),
        crest: Range::new(8.0, 10.0),
        low: Range::new(50.0, 66.0),
        mid: Range::new(32.0, 45.0),
        high: Range::new(2.0, 3.0),
        correlation: None,
        aim_crest: Some(9.0),
        opening_under_db: 6.0,
    },
    Reference {
        id: "bfg-division",
        row: "BFG Division",
        mean: Range::new(-14.5, -10.5),
        crest: Range::new(11.5, 15.0),
        low: Range::new(54.0, 82.0),
        mid: Range::new(16.0, 43.0),
        high: Range::new(1.0, 4.0),
        correlation: Some(Range::new(0.45, 0.6)),
        aim_crest: Some(12.5),
        opening_under_db: -1.5,
    },
    Reference {
        id: "brutal-doom",
        row: "Brutal Doom",
        mean: Range::new(-13.0, -11.0),
        crest: Range::new(11.5, 13.5),
        low: Range::new(73.0, 85.0),
        mid: Range::new(14.0, 26.0),
        high: Range::new(1.0, 2.0),
        correlation: Some(Range::new(0.5, 0.8)),
        aim_crest: Some(12.0),
        opening_under_db: 3.5,
    },
];

/// The reference a caller named, or a sentence listing the ones there are.
///
/// # Errors
///
/// When `id` names none of [`REFERENCES`].
pub fn find(id: &str) -> Result<&'static Reference, String> {
    REFERENCES
        .iter()
        .find(|reference| reference.id == id)
        .ok_or_else(|| {
            let ids: Vec<&str> = REFERENCES.iter().map(|reference| reference.id).collect();
            format!(
                "no reference record `{id}` — the ones measured are {} (`guide references`)",
                ids.join(", ")
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{REFERENCES, find};

    #[test]
    fn a_reference_is_found_by_its_id_and_a_wrong_one_lists_the_rest() {
        assert_eq!(find("billie-jean").map(|it| it.row), Ok("Billie Jean"));
        let wrong = find("thriller").expect_err("not measured");
        assert!(wrong.contains("`thriller`") && wrong.contains("brutal-doom"));
    }

    #[test]
    fn every_range_runs_low_to_high() {
        for reference in REFERENCES {
            let ranges = [
                reference.mean,
                reference.crest,
                reference.low,
                reference.mid,
                reference.high,
            ];
            for range in ranges.iter().chain(&reference.correlation) {
                assert!(range.low <= range.high, "{}: {range:?}", reference.id);
            }
        }
    }

    #[test]
    fn the_reachable_mean_is_one_under_the_ceiling_minus_the_crest() {
        let billie = find("billie-jean").expect("measured");
        assert_eq!(billie.reachable_mean(), Some(-16.5));
        assert_eq!(
            find("fuego-breaks").expect("measured").reachable_mean(),
            None
        );
    }
}
