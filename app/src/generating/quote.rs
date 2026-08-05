//! What a run would cost, read out of the document before anyone commits to it.
//!
//! **Every figure here is an estimate, and the words on screen say so.** No
//! provider scorsese talks to reports what a generation cost — a finished Veo
//! operation carries a video link and nothing else — so a project total is our
//! own arithmetic over a published rate table. A window is the one place a
//! person reads that number before deciding to spend more, which makes it the
//! place it must not read as a receipt. See `docs/prices.md`.

use scorsese_core::{AssetId, AssetKind, GenerationState, Project};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::{dollars, estimate};

/// One shot that would be paid for.
pub(crate) struct Shot {
    /// Which asset.
    pub(crate) asset: AssetId,
    /// What it is estimated to cost, in US cents.
    pub(crate) cents: u64,
    /// How it reads: `8s of Fast at 1080p`.
    pub(crate) shape: String,
}

/// What a run would cost, and what room there is for it.
pub(crate) struct Quote {
    /// The shots that would be sent — sketches and stale briefs, never
    /// something already generated.
    pub(crate) shots: Vec<Shot>,
    /// What they add up to, in US cents.
    pub(crate) total_cents: u64,
    /// What the project already says has been spent on it.
    pub(crate) spent_cents: u64,
    /// The ceiling, when somebody set one.
    pub(crate) ceiling_cents: Option<u64>,
    /// How many shots are already in flight from an earlier run.
    pub(crate) in_flight: usize,
}

impl Quote {
    /// Reads what this project would cost to realise.
    pub(crate) fn of(project: &Project) -> Self {
        let mut shots = Vec::new();
        let mut in_flight = 0;
        // `generated_video` and not merely "prompted". A narration prompt is
        // also a prompt, and pricing one through a *video* rate table quotes an
        // eight-second shot for a line of speech — a total that is wrong
        // upward, about money, and looks perfectly reasonable on screen.
        // ElevenLabs has no rate here yet, and until it does, saying nothing
        // about it beats inventing a number.
        for asset in project
            .assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::GeneratedVideo)
        {
            if asset.operation.is_some() {
                in_flight += 1;
                continue;
            }
            // Anything already generated is not sent again and so costs
            // nothing — the cache is what makes that true, and quoting for it
            // would be quoting for work already paid for.
            if asset.state == Some(GenerationState::Generated) {
                continue;
            }
            let request = asset.video_request();
            let Ok(priced) = estimate(&request) else {
                continue;
            };
            shots.push(Shot {
                asset: asset.id.clone(),
                cents: priced.cents,
                shape: format!(
                    "{}s of {} at {}",
                    priced.seconds,
                    request.model.as_str(),
                    request.resolution.as_str()
                ),
            });
        }
        let settings = Settings::load().unwrap_or_default();
        Self {
            total_cents: shots.iter().map(|shot| shot.cents).sum(),
            shots,
            spent_cents: spent(project),
            ceiling_cents: settings.budget_cents,
            in_flight,
        }
    }

    /// The total, as a person reads money.
    pub(crate) fn total(&self) -> String {
        dollars(self.total_cents)
    }

    /// Whether this run would cross the ceiling — asked of [`Budget`], the same
    /// check `scorsese generate` makes, so the window cannot say yes to a run
    /// the library would refuse.
    pub(crate) fn over_budget(&self) -> bool {
        Budget::new(self.ceiling_cents.unwrap_or(u64::MAX), self.spent_cents)
            .check(self.total_cents)
            .is_err()
    }

    /// The line about the ceiling, or `None` when nobody set one.
    pub(crate) fn ceiling(&self) -> Option<String> {
        let ceiling = self.ceiling_cents?;
        Some(format!(
            "{} spent of a {} ceiling",
            dollars(self.spent_cents),
            dollars(ceiling)
        ))
    }
}

/// What the assets already say has been spent on them.
///
/// Summed off the assets rather than kept as a running tally, so deleting a
/// shot takes its cost with it — a ledger elsewhere would be wrong the first
/// time anybody removed one.
pub(crate) fn spent(project: &Project) -> u64 {
    project
        .assets
        .iter()
        .filter_map(|asset| asset.estimated_cost_cents)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::Quote;
    use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project};

    /// A project with one shot to make and one line of narration.
    fn project() -> Project {
        let mut project = Project::new("quoting", Fps::THIRTY);
        project.assets.push(Asset::sketch(
            AssetId::new("shot"),
            AssetKind::GeneratedVideo,
            "a city at dawn",
        ));
        project.assets.push(Asset::sketch(
            AssetId::new("vo"),
            AssetKind::GeneratedAudio,
            "In a city that never sleeps.",
        ));
        project
    }

    /// The bug this was found with, and the reason it is worth a test: a
    /// narration prompt is a prompt, so quoting everything "prompted" priced a
    /// line of speech as an eight-second video shot — and doubled the total a
    /// person was about to agree to.
    #[test]
    fn narration_is_not_quoted_as_a_video_shot() {
        let quote = Quote::of(&project());
        assert_eq!(quote.shots.len(), 1, "only the video shot has a price");
        assert_eq!(quote.shots[0].asset.as_str(), "shot");
        assert_eq!(
            quote.total_cents, 96,
            "eight seconds of Fast at 1080p, and nothing for the narration"
        );
    }

    /// No ceiling refuses nothing — a limit nobody set is not a limit.
    #[test]
    fn a_run_with_no_ceiling_is_never_over_budget() {
        assert!(!Quote::of(&project()).over_budget());
    }
}
