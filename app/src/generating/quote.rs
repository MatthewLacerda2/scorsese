//! What a run would cost, read out of the document before anyone commits to it.
//!
//! **Every figure here is an estimate, and the words on screen say so.** No
//! provider scorsese talks to reports what a generation cost — a finished Veo
//! operation carries a video link and nothing else — so a project total is our
//! own arithmetic over a published rate table. A window is the one place a
//! person reads that number before deciding to spend more, which makes it the
//! place it must not read as a receipt. See `docs/prices.md`.
//!
//! # Two rate tables, and never one filter
//!
//! Shots and lines are counted in **separate passes over separate asset
//! kinds**, and that separation is the whole design of this module rather than
//! a tidiness. Quoting everything merely *prompted* is what priced a line of
//! narration as an eight-second 1080p shot — ninety-six cents for something
//! that costs two, in a total somebody was about to agree to, and it looked
//! entirely reasonable on screen. `narration_is_not_quoted_as_a_video_shot` is
//! that bug, kept.
//!
//! So the [`AssetKind::GeneratedVideo`] filter is never widened. Speech arrived
//! as its own pass with its own arithmetic beside it, and a third kind would
//! arrive the same way.

use scorsese_core::{AssetId, AssetKind, GenerationState, Project};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::{dollars, estimate};
use scorsese_providers::speech;

/// One generation that would be paid for.
pub(crate) struct Priced {
    /// Which asset.
    pub(crate) asset: AssetId,
    /// What it is estimated to cost, in US cents.
    pub(crate) cents: u64,
    /// How it reads: `8s of Fast at 1080p`, `137 characters in standard`.
    pub(crate) shape: String,
}

/// What a run would cost, and what room there is for it.
pub(crate) struct Quote {
    /// The shots that would be sent — sketches and stale briefs, never
    /// something already generated.
    pub(crate) shots: Vec<Priced>,
    /// The lines that would be spoken, on the same terms.
    pub(crate) lines: Vec<Priced>,
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
        let (shots, in_flight) = shots(project);
        let lines = lines(project);
        let settings = Settings::load().unwrap_or_default();
        Self {
            total_cents: shots
                .iter()
                .chain(lines.iter())
                .map(|priced| priced.cents)
                .sum(),
            shots,
            lines,
            spent_cents: spent(project),
            ceiling_cents: settings.budget_cents,
            in_flight,
        }
    }

    /// Whether there is anything at all to make.
    pub(crate) fn empty(&self) -> bool {
        self.shots.is_empty() && self.lines.is_empty()
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

/// The shots to be made, and how many are already going.
///
/// `generated_video` and not merely "prompted" — see the module doc for what
/// widening this once cost.
fn shots(project: &Project) -> (Vec<Priced>, usize) {
    let mut shots = Vec::new();
    let mut in_flight = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedVideo)
    {
        if asset.operation.is_some() {
            in_flight += 1;
            continue;
        }
        // Anything already generated is not sent again and so costs nothing —
        // the cache is what makes that true, and quoting for it would be
        // quoting for work already paid for.
        if asset.state == Some(GenerationState::Generated) {
            continue;
        }
        let request = asset.video_request();
        let Ok(priced) = estimate(&request) else {
            continue;
        };
        shots.push(Priced {
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
    (shots, in_flight)
}

/// The lines to be spoken.
///
/// Nothing here is ever *in flight*: a reading comes back on the connection it
/// was asked for, so there is no third state between quoted and made.
///
/// Priced from the words already in the document, so unlike a shot the figure
/// is exact before anything is sent — the vendor bills per character. It is
/// still an estimate: the rate is a page somebody copied, and library-voice
/// multipliers are deliberately ignored.
fn lines(project: &Project) -> Vec<Priced> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedAudio)
        .filter(|asset| asset.state != Some(GenerationState::Generated))
        .map(|asset| match speech::Brief::of(asset) {
            Ok(brief) => Priced {
                asset: asset.id.clone(),
                cents: speech::quote(brief.request.model, brief.characters()),
                shape: format!(
                    "{} characters in {}",
                    brief.characters(),
                    brief.request.model.as_str()
                ),
            },
            // Asked of the same function the run asks, so this dialog cannot
            // quote a line the run would skip. A half-written line costs
            // nothing and is listed anyway — a total that silently omitted it
            // would leave somebody wondering which of their narrations it
            // covered.
            Err(why) => Priced {
                cents: 0,
                shape: unprefixed(&why.to_string(), &asset.id),
                asset: asset.id.clone(),
            },
        })
        .collect()
}

/// A sentence with the asset's own name taken off the front.
///
/// The library names the asset because a terminal prints one line per problem;
/// this dialog has already put the id in the row, and saying it twice reads as
/// a stutter.
fn unprefixed(said: &str, asset: &AssetId) -> String {
    said.strip_prefix(&format!("{asset} "))
        .unwrap_or(said)
        .to_owned()
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
    use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, SpeechRequest};

    /// A project with one shot to make and one line of narration.
    fn project() -> Project {
        let mut project = Project::new("quoting", Fps::THIRTY);
        project.assets.push(Asset::sketch(
            AssetId::new("shot"),
            AssetKind::GeneratedVideo,
            "a city at dawn",
        ));
        let mut line = Asset::sketch(
            AssetId::new("vo"),
            AssetKind::GeneratedAudio,
            "In a city that never sleeps.",
        );
        line.speech = Some(SpeechRequest {
            voice_id: Some(String::from("a-voice")),
            ..SpeechRequest::default()
        });
        project.assets.push(line);
        project
    }

    /// The bug this was found with, and the reason it is worth a test: a
    /// narration prompt is a prompt, so quoting everything "prompted" priced a
    /// line of speech as an eight-second video shot — and doubled the total a
    /// person was about to agree to.
    ///
    /// It still means that now narration *is* priced. The shot list holds the
    /// shot and nothing else, and the line is quoted through the speech table
    /// at a cent rather than through the video table at ninety-six.
    #[test]
    fn narration_is_not_quoted_as_a_video_shot() {
        let quote = Quote::of(&project());
        assert_eq!(quote.shots.len(), 1, "only the video shot is a shot");
        assert_eq!(quote.shots[0].asset.as_str(), "shot");
        assert_eq!(quote.shots[0].cents, 96, "eight seconds of Fast at 1080p");

        assert_eq!(quote.lines.len(), 1);
        assert_eq!(quote.lines[0].asset.as_str(), "vo");
        assert_eq!(
            quote.lines[0].cents, 1,
            "28 characters at 10c per 1000, rounded up — never the video rate"
        );
        assert_eq!(quote.total_cents, 97);
    }

    /// A line nobody has chosen a voice for is listed and costs nothing, in the
    /// words the run itself would use. Dropping it from the dialog would leave
    /// a person wondering which of their narrations the total covered.
    #[test]
    fn a_line_that_is_not_ready_is_quoted_at_nothing_and_says_why() {
        let mut project = project();
        project.assets[1].speech = None;

        let quote = Quote::of(&project);
        assert_eq!(quote.lines[0].cents, 0);
        assert!(
            quote.lines[0].shape.contains("no voice"),
            "{:?}",
            quote.lines[0].shape
        );
        assert!(
            !quote.lines[0].shape.starts_with("vo "),
            "the row already names the asset"
        );
        assert_eq!(quote.total_cents, 96);
    }

    /// No ceiling refuses nothing — a limit nobody set is not a limit.
    #[test]
    fn a_run_with_no_ceiling_is_never_over_budget() {
        assert!(!Quote::of(&project()).over_budget());
    }
}
