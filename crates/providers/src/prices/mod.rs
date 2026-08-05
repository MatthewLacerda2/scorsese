//! What a generation will cost, before anyone spends it.
//!
//! # The estimate is ours, not a bill
//!
//! **No provider here tells us what a generation cost.** Veo's operation comes
//! back carrying a video URI and nothing else — no billed amount, no usage
//! record, no token count. Google reports spend at the project level, a day
//! late, through Cloud Billing; nothing on the generative API's own responses
//! attributes a cent to the shot that incurred it.
//!
//! So every number this module produces is *our own arithmetic over the table
//! in [`veo`]*, computed at the moment of generating, and that is why the field
//! it is written into is called `estimated_cost_cents` rather than anything
//! shorter. The shorter name was the original one, and it read as a bill. A
//! project total is a sum of these, shown to somebody deciding whether to
//! spend more — a number labelled *what this cost* that actually means *what
//! we worked out it would cost* is exactly the quiet wrongness worth spelling
//! out in the name.
//!
//! What follows from that, and holds wherever a total is printed: the estimate
//! is right when the table is right, and the table is a page somebody copied.
//! [`Checked`] is the only defence, and it is a signal rather than a gate —
//! a figure going unread does not make it wrong.
//!
//! # Cents, and only cents
//!
//! Whole US cents throughout, as in [`Budget`](crate::credentials::Budget),
//! and for the same reason: a total is a sum, and a sum of floats is not the
//! same number twice. Every rate Veo publishes happens to be a whole number of
//! cents per second, so nothing rounds on the way in either.

pub mod checked;
pub mod veo;

pub use checked::{Checked, STALE_AFTER_DAYS};
pub use veo::{Quality, Rate, Tier};

use scorsese_core::VideoRequest;

/// What a generation is expected to cost, and what that was worked out from.
///
/// Carries the rate and the length as well as the total, because an estimate
/// somebody is about to approve should be checkable without re-deriving it —
/// *8 seconds at 12¢ is 96¢* is an argument, and `96` on its own is a demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The total, in US cents. **Our calculation, never a billed figure.**
    pub cents: u64,
    /// The rate it came from, date included.
    pub rate: Rate,
    /// How many seconds of video were priced.
    pub seconds: u32,
}

/// A combination the vendor does not sell, so there is no price to quote.
///
/// Its own error rather than a `None`, because the honest answer to *what will
/// this cost* is sometimes *nobody sells that*, and collapsing it into "zero"
/// or "unknown" would let a run go ahead on a number that never existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{} is not sold at {} — there is no price to quote", tier.label(), quality.label())]
pub struct Unpriced {
    /// The tier asked for.
    pub tier: Tier,
    /// The output size asked for.
    pub quality: Quality,
}

/// What generating this shot is expected to cost.
///
/// The one answer to that question: `scorsese generate` prints it, the budget
/// ceiling is checked against it, and it is what gets recorded on the asset
/// once the shot exists. Three callers, one calculation — a second one would
/// be a second answer, and the first person to notice they disagree would be
/// the person who had already paid.
pub fn estimate(request: &VideoRequest) -> Result<Estimate, Unpriced> {
    let tier = Tier::from(request.model);
    let quality = Quality::from(request.resolution);
    let rate = veo::rate(tier, quality).ok_or(Unpriced { tier, quality })?;
    let seconds = request.seconds.get();
    Ok(Estimate {
        cents: rate.cents_per_second * u64::from(seconds),
        rate,
        seconds,
    })
}

/// Cents as the dollars-and-cents string a person expects to read.
///
/// Integer arithmetic on the way out as well as in: `$0.05` is a division and
/// a remainder, not a float that prints as `0.05000000000000001`.
pub fn dollars(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}
