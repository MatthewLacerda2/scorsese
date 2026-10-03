//! The live provider check (#567): real calls, on purpose, by a person.
//!
//! Every client under [`crate::api`] is plain HTTP somebody here wrote, and
//! the tests prove only that the code agrees with **our fixtures**. Nothing
//! else notices a vendor changing its API, or a change of ours drifting from
//! what the vendor accepts — and on the hosted web app that failure is a
//! paying user's generation breaking. This module makes the smallest real
//! call to each vendor that exercises the path scorsese depends on, through
//! the product's own clients, and says per vendor: **OK**, **shape changed**
//! (naming the field), **refused**, **auth failed**, or **skipped**.
//!
//! It spends money and needs a network, so it is never a test and never a
//! gate — `CLAUDE.md`'s *no real provider calls in tests, ever*. It is a
//! signal somebody runs before a deploy or after a vendor moves a model:
//! `scorsese check-providers`, which quotes first, asks, and is held to the
//! same `budget_cents` ceiling as any other spend. `docs/live-check.md` has
//! how to run it and what each call costs.
//!
//! Keys come from the one resolver ([`crate::credentials`]); a vendor with no
//! key is **skipped**, never failed. Every reply is copied through a
//! [`Tap`] and scrubbed of the key ([`record`]), so a run can leave behind the
//! real bodies that replace a hand-written fixture.

pub mod claude;
pub mod elevenlabs;
pub mod image;
pub mod judge;
pub mod record;
pub mod veo;

use std::time::Duration;

use crate::api::tap::{Exchange, Tap};
use crate::credentials::{
    Budget, Environment, OverBudget, Provider, Secret, Settings, resolve_from,
};

/// Every vendor the check calls, in the order it calls them.
pub const PROVIDERS: [Provider; 3] = [Provider::Gemini, Provider::ElevenLabs, Provider::Anthropic];

/// What a person chose about this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Pay for one real Veo shot — see [`veo`] for why that is opt-in.
    pub include_veo: bool,
    /// How long to wait for that shot before giving up on watching it.
    pub veo_patience: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            include_veo: false,
            veo_patience: Duration::from_secs(600),
        }
    }
}

/// What one call found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The reply was the shape our client reads.
    Ok,
    /// The reply was not: `field` names what was missing or different.
    ShapeChanged {
        /// What changed, as specifically as the reply allowed.
        field: String,
    },
    /// The vendor said no for a reason other than the key.
    Refused {
        /// The vendor's words.
        said: String,
    },
    /// The vendor said no to the key.
    AuthFailed {
        /// The vendor's words.
        said: String,
    },
    /// No answer arrived at all.
    Unreachable {
        /// What the transport said.
        said: String,
    },
    /// The call worked but stopped short of what could be checked.
    Unfinished {
        /// What was left unchecked, and why.
        said: String,
    },
    /// Not called.
    Skipped {
        /// Why.
        why: String,
    },
}

impl Verdict {
    /// The word a report prints.
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::ShapeChanged { .. } => "shape changed",
            Self::Refused { .. } => "refused",
            Self::AuthFailed { .. } => "auth failed",
            Self::Unreachable { .. } => "unreachable",
            Self::Unfinished { .. } => "unfinished",
            Self::Skipped { .. } => "skipped",
        }
    }

    /// The detail a report prints after the word; empty for [`Verdict::Ok`].
    pub fn detail(&self) -> &str {
        match self {
            Self::Ok => "",
            Self::ShapeChanged { field } => field,
            Self::Refused { said }
            | Self::AuthFailed { said }
            | Self::Unreachable { said }
            | Self::Unfinished { said } => said,
            Self::Skipped { why } => why,
        }
    }

    /// Whether this is something to fix: a skip or an unfinished call is not.
    pub const fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::ShapeChanged { .. }
                | Self::Refused { .. }
                | Self::AuthFailed { .. }
                | Self::Unreachable { .. }
        )
    }
}

/// One call and what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The call, as a person would name it.
    pub call: String,
    /// What it found.
    pub verdict: Verdict,
    /// Anything worth saying about a call that passed.
    pub notes: Vec<String>,
}

impl Step {
    /// A step with no notes.
    pub fn new(call: impl Into<String>, verdict: Verdict) -> Self {
        Self {
            call: call.into(),
            verdict,
            notes: Vec::new(),
        }
    }

    /// The same step, with a note.
    pub fn noting(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// What the check will do with one vendor, before it does it.
#[derive(Debug, Clone)]
pub struct Planned {
    /// The vendor.
    pub provider: Provider,
    /// Why it will not be called, when it will not.
    pub skipped: Option<String>,
    /// Each call, with what it costs.
    pub calls: Vec<String>,
    /// The most it will spend, in US cents. Our arithmetic, never a bill.
    pub cents: u64,
    key: Option<Secret>,
}

/// What the check will do, vendor by vendor. Pure: the keys come out of the
/// two values the one resolver reads, and nothing is called.
pub fn plan(options: &Options, environment: &Environment, settings: &Settings) -> Vec<Planned> {
    PROVIDERS
        .iter()
        .map(
            |&provider| match resolve_from(provider, environment, settings) {
                Ok(found) => Planned {
                    provider,
                    skipped: None,
                    calls: calls(provider, options),
                    cents: cost(provider, options),
                    key: Some(found.secret),
                },
                Err(error) => Planned {
                    provider,
                    // `Missing` already reads "no key for …: looked in …".
                    skipped: Some(error.to_string()),
                    calls: Vec::new(),
                    cents: 0,
                    key: None,
                },
            },
        )
        .collect()
}

/// The calls one vendor's part makes.
fn calls(provider: Provider, options: &Options) -> Vec<String> {
    match provider {
        Provider::Gemini => [veo::calls(options), image::calls()].concat(),
        Provider::ElevenLabs => elevenlabs::calls(),
        Provider::Anthropic => claude::calls(),
    }
}

/// The most one vendor's part spends.
fn cost(provider: Provider, options: &Options) -> u64 {
    match provider {
        Provider::Gemini => veo::cost(options) + image::cost(),
        Provider::ElevenLabs => elevenlabs::cost(),
        Provider::Anthropic => claude::cost(),
    }
}

/// The most the whole plan spends, in US cents.
pub fn total(plan: &[Planned]) -> u64 {
    plan.iter().map(|planned| planned.cents).sum()
}

/// Whether the plan fits under the ceiling. Asked by [`run`] before anything
/// is sent, so no flag and no caller gets past it.
pub fn permit(plan: &[Planned], budget: Budget) -> Result<(), OverBudget> {
    budget.check(total(plan))
}

/// What the check found for one vendor.
#[derive(Debug, Clone)]
pub struct Report {
    /// The vendor.
    pub provider: Provider,
    /// Each call, in order.
    pub steps: Vec<Step>,
    /// What it is estimated to have spent, in US cents — Claude's from the
    /// vendor's own token counts. Never a bill.
    pub cents: u64,
    /// Every reply received, scrubbed of the key and account ids.
    pub exchanges: Vec<Exchange>,
}

impl Report {
    /// The vendor's verdict: its first failure, else its first unfinished
    /// call, else OK if anything was checked, else the skip.
    pub fn verdict(&self) -> &Verdict {
        let first =
            |test: fn(&Verdict) -> bool| self.steps.iter().map(|s| &s.verdict).find(|v| test(v));
        first(Verdict::is_failure)
            .or_else(|| first(|v| matches!(v, Verdict::Unfinished { .. })))
            .or_else(|| first(|v| matches!(v, Verdict::Ok)))
            .or_else(|| self.steps.first().map(|s| &s.verdict))
            .unwrap_or(&Verdict::Ok)
    }
}

/// Runs the plan: every vendor with a key, in order. `on` hears progress
/// while a call takes a while.
///
/// Refused whole, before anything is sent, if the plan would cross `budget`.
pub fn run(
    plan: &[Planned],
    options: &Options,
    budget: Budget,
    on: &mut dyn FnMut(&str),
) -> Result<Vec<Report>, OverBudget> {
    permit(plan, budget)?;
    Ok(plan
        .iter()
        .map(|planned| one(planned, options, on))
        .collect())
}

/// One vendor's part.
fn one(planned: &Planned, options: &Options, on: &mut dyn FnMut(&str)) -> Report {
    let (Some(key), None) = (&planned.key, &planned.skipped) else {
        let why = planned.skipped.clone().unwrap_or_default();
        return Report {
            provider: planned.provider,
            steps: vec![Step::new("every call", Verdict::Skipped { why })],
            cents: 0,
            exchanges: Vec::new(),
        };
    };
    on(&format!("Checking {}…", planned.provider.label()));
    let tap = Tap::new();
    let (steps, cents) = match planned.provider {
        Provider::Gemini => {
            let (mut steps, cents) = veo::check(key, &tap, options, on);
            let (still, spent) = image::check(key, &tap);
            steps.extend(still);
            (steps, cents + spent)
        }
        Provider::ElevenLabs => elevenlabs::check(key, &tap),
        Provider::Anthropic => claude::check(key, &tap),
    };
    let exchanges = tap
        .take()
        .into_iter()
        .map(|exchange| Exchange {
            body: record::scrub(&exchange.body, key.expose()),
            ..exchange
        })
        .collect();
    Report {
        provider: planned.provider,
        steps,
        cents,
        exchanges,
    }
}
