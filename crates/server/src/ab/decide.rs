//! When a landing test is decided (#1553): the 50 / 90% / 100 rule.
//!
//! Every version needs [`FLOOR`] visitors before anything is said. Then a
//! version that has at least a [`CONFIDENT`] chance of being the best on the
//! prompt rate, or else on reaching the last section, wins. At [`CEILING`]
//! visitors per version with no such version the test is ready anyway, as
//! *no clear difference*. Nothing waits for more than [`CEILING`].
//!
//! The chance is the posterior of a Beta-binomial with a uniform prior: each
//! version's rate is `Beta(1 + hits, 1 + misses)`, and the chance a version
//! is the best is the probability its rate is above every other's, integrated
//! on a fixed grid, so the same counts always give the same answer.

/// Visitors every version needs before a test can be decided.
pub const FLOOR: u64 = 50;

/// Visitors per version at which a test is decided whatever the counts say.
pub const CEILING: u64 = 100;

/// The chance of being the best that makes a version the winner.
pub const CONFIDENT: f64 = 0.9;

/// Points the posteriors are integrated over.
const GRID: usize = 2000;

/// What one version of a test has seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arm {
    /// The version's id.
    pub version: String,
    /// Visitors shown it.
    pub visitors: u64,
    /// Of them, those who sent a prompt.
    pub prompts: u64,
    /// Of them, those who reached the last section.
    pub ends: u64,
}

/// Which signal decided a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// The share of visitors who sent a prompt.
    Prompt,
    /// The share of visitors who reached the last section.
    LastSection,
}

/// What the rule says of a test.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Not yet: every version needs `more` visitors before the rule is
    /// applied again (to the floor, or else to the ceiling).
    NotReady {
        /// Visitors the version with fewest still needs.
        more: u64,
    },
    /// Ready: `version` wins on `on`, with `chance` of being the best.
    Winner {
        /// The winning version's id.
        version: String,
        /// The signal it wins on.
        on: Signal,
        /// Its chance of being the best on that signal.
        chance: f64,
    },
    /// Ready at the ceiling with no version confidently best: keep the one
    /// preferred, the first by default.
    NoClearDifference,
}

/// The rule applied to `arms`, the versions of one test.
pub fn verdict(arms: &[Arm]) -> Verdict {
    let fewest = arms.iter().map(|arm| arm.visitors).min().unwrap_or(0);
    if arms.len() < 2 || fewest < FLOOR {
        return Verdict::NotReady {
            more: FLOOR.saturating_sub(fewest).max(1),
        };
    }
    for (on, hits) in [
        (Signal::Prompt, (|arm: &Arm| arm.prompts) as fn(&Arm) -> u64),
        (Signal::LastSection, |arm: &Arm| arm.ends),
    ] {
        let counts: Vec<_> = arms.iter().map(|arm| (hits(arm), arm.visitors)).collect();
        let chances = chance_best(&counts);
        if let Some((arm, &chance)) = arms
            .iter()
            .zip(&chances)
            .find(|(_, chance)| **chance >= CONFIDENT)
        {
            return Verdict::Winner {
                version: arm.version.clone(),
                on,
                chance,
            };
        }
    }
    if fewest >= CEILING {
        Verdict::NoClearDifference
    } else {
        Verdict::NotReady {
            more: CEILING - fewest,
        }
    }
}

/// For each `(hits, trials)`, the chance its rate is the highest.
pub fn chance_best(counts: &[(u64, u64)]) -> Vec<f64> {
    let densities: Vec<Vec<f64>> = counts
        .iter()
        .map(|&(hits, trials)| density(hits, trials.max(hits)))
        .collect();
    // Below each grid point, and half of the point itself.
    let below: Vec<Vec<f64>> = densities
        .iter()
        .map(|density| {
            let mut sum = 0.0;
            density
                .iter()
                .map(|mass| {
                    let half = sum + mass / 2.0;
                    sum += mass;
                    half
                })
                .collect()
        })
        .collect();
    (0..counts.len())
        .map(|best| {
            (0..GRID)
                .map(|point| {
                    densities[best][point]
                        * (0..counts.len())
                            .filter(|&other| other != best)
                            .map(|other| below[other][point])
                            .product::<f64>()
                })
                .sum()
        })
        .collect()
}

/// `Beta(1 + hits, 1 + trials - hits)` on the grid's midpoints, summing to 1.
fn density(hits: u64, trials: u64) -> Vec<f64> {
    let (a, b) = (hits as f64, (trials - hits) as f64);
    let logs: Vec<f64> = (0..GRID)
        .map(|point| {
            let x = (point as f64 + 0.5) / GRID as f64;
            a * x.ln() + b * (1.0 - x).ln()
        })
        .collect();
    let peak = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let masses: Vec<f64> = logs.iter().map(|log| (log - peak).exp()).collect();
    let total: f64 = masses.iter().sum();
    masses.iter().map(|mass| mass / total).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(version: &str, visitors: u64, prompts: u64, ends: u64) -> Arm {
        Arm {
            version: version.into(),
            visitors,
            prompts,
            ends,
        }
    }

    #[test]
    fn equal_counts_are_an_even_chance() {
        for chance in chance_best(&[(3, 50), (3, 50)]) {
            assert!((chance - 0.5).abs() < 1e-9, "{chance}");
        }
        let three = chance_best(&[(0, 0), (0, 0), (0, 0)]);
        assert!(
            three.iter().all(|chance| (chance - 1.0 / 3.0).abs() < 1e-3),
            "{three:?}"
        );
    }

    #[test]
    fn the_chance_follows_the_counts() {
        // Beta(11, 41) against Beta(3, 49): P(A > B) is about 0.99.
        let [a, b] = chance_best(&[(10, 50), (2, 50)])[..] else {
            panic!("two chances")
        };
        assert!(a > 0.98 && a < 0.999, "{a}");
        assert!((a + b - 1.0).abs() < 1e-6, "{a} + {b}");
    }

    #[test]
    fn forty_nine_in_one_version_is_not_ready() {
        let verdict = verdict(&[arm("a", 49, 10, 30), arm("b", 50, 0, 5)]);
        assert_eq!(verdict, Verdict::NotReady { more: 1 });
    }

    #[test]
    fn fifty_each_with_a_confident_leader_is_ready() {
        let Verdict::Winner {
            version,
            on,
            chance,
        } = verdict(&[arm("a", 50, 1, 20), arm("b", 50, 6, 20)])
        else {
            panic!("a winner")
        };
        assert_eq!((version.as_str(), on), ("b", Signal::Prompt));
        assert!(chance >= CONFIDENT, "{chance}");
    }

    #[test]
    fn reaching_the_last_section_decides_when_prompts_do_not() {
        let verdict = verdict(&[arm("a", 60, 2, 30), arm("b", 60, 2, 15)]);
        assert!(
            matches!(verdict, Verdict::Winner { ref version, on: Signal::LastSection, .. } if version == "a"),
            "{verdict:?}"
        );
    }

    #[test]
    fn fifty_each_without_a_leader_is_not_ready() {
        let verdict = verdict(&[arm("a", 50, 2, 20), arm("b", 52, 3, 21)]);
        assert_eq!(verdict, Verdict::NotReady { more: 50 });
    }

    #[test]
    fn a_hundred_each_without_a_leader_is_no_clear_difference() {
        let verdict = verdict(&[arm("a", 100, 5, 40), arm("b", 100, 5, 41)]);
        assert_eq!(verdict, Verdict::NoClearDifference);
    }

    #[test]
    fn one_version_alone_is_never_decided() {
        assert!(matches!(
            verdict(&[arm("a", 500, 50, 50)]),
            Verdict::NotReady { .. }
        ));
    }
}
