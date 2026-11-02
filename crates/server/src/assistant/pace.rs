//! One member's share of the model's rate limits (#1653).
//!
//! Every call goes out on the operator's one key, so every member draws on
//! the organisation's one set of per-minute limits, model by model. Input
//! tokens are the one a long conversation reaches first, so each member has
//! a budget of them per minute and model ([`budget`]): once the calls they
//! made in the last minute have sent that many, their next call waits until
//! enough of those calls are a minute old. Another member's calls never
//! wait on theirs.
//!
//! It counts what the organisation's limit counts: the input of each reply's
//! usage that was not read from the cache — plain input and cache writes
//! (Anthropic's *Rate limits* page, read 2026-11-02: "only `input_tokens` +
//! `cache_creation_input_tokens` count toward your ITPM limit"). A call is
//! counted once it has answered, so a member past the budget is past it by
//! at most their calls in flight, which `holds` bounds at a few.
//!
//! In memory, like the holds: there is one server, and a restart forgets a
//! minute nobody needs remembered. A wait is a minute at most, and a Stop
//! asked for during it ends it within [`HEARD_WITHIN`].

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use scorsese_providers::chat::{Model, Usage};

use crate::db::UserId;

/// How long a call's input counts against the member's budget.
const WINDOW: Duration = Duration::from_secs(60);

/// The longest a wait goes without looking whether its turn was stopped.
const HEARD_WITHIN: Duration = Duration::from_secs(1);

/// How many uncached input tokens one member may send `model` in a minute:
/// a quarter of the organisation's limit for it on Anthropic's lowest
/// standard tier, Start (*Rate limits* page, read 2026-11-02: 2,000,000 input
/// tokens a minute on Claude Opus 5.5 and on Claude Sonnet 5.5, 500,000 on
/// Claude Fable 5.x). A quarter, so one member is never most of it.
pub(super) const fn budget(model: Model) -> u64 {
    match model {
        Model::ClaudeOpus55 | Model::ClaudeSonnet55 => 500_000,
        Model::ClaudeFable51 => 125_000,
    }
}

/// The input each member's calls sent in the last minute, by member and
/// model: when each answered, and how many tokens counted. Cheap to clone;
/// clones share it.
#[derive(Clone, Default)]
pub(super) struct Pace(Arc<Mutex<HashMap<(UserId, Model), Sent>>>);

/// One member's calls on one model in the last minute, oldest first: when
/// each answered, and the input tokens it counted.
type Sent = VecDeque<(Instant, u64)>;

impl Pace {
    /// Wait until `user` is under their budget on `model`: `true` then, or
    /// `false` as soon as `stopped` says the turn waiting was stopped.
    pub(super) async fn wait(
        &self,
        user: UserId,
        model: Model,
        stopped: impl Fn() -> bool,
    ) -> bool {
        let mut told = false;
        while let Some(until) = self.free_at(user, model, Instant::now()) {
            if stopped() {
                return false;
            }
            if !told {
                tracing::info!(
                    user = user.get(),
                    "assistant: past the input budget on {}; waiting",
                    model.label()
                );
                told = true;
            }
            let next = until.min(Instant::now() + HEARD_WITHIN);
            tokio::time::sleep_until(next.into()).await;
        }
        true
    }

    /// Count a call of `user`'s on `model` that answered with `usage`.
    pub(super) fn sent(&self, user: UserId, model: Model, usage: &Usage) {
        let counted = usage
            .input
            .saturating_add(usage.cache_write_5m)
            .saturating_add(usage.cache_write_1h);
        let mut sent = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        sent.entry((user, model))
            .or_default()
            .push_back((Instant::now(), counted));
    }

    /// When `user` is next under their budget on `model`, or `None` when
    /// they are at `now`. Forgets what is a minute old.
    fn free_at(&self, user: UserId, model: Model, now: Instant) -> Option<Instant> {
        let mut sent = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let calls = sent.get_mut(&(user, model))?;
        while calls
            .front()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) >= WINDOW)
        {
            calls.pop_front();
        }
        let mut counted = calls.iter().map(|(_, tokens)| tokens).sum::<u64>();
        if calls.is_empty() {
            sent.remove(&(user, model));
            return None;
        }
        let budget = budget(model);
        if counted < budget {
            return None;
        }
        // The oldest calls age out first: under budget once enough have.
        calls.iter().find_map(|(at, tokens)| {
            counted -= tokens;
            (counted < budget).then_some(*at + WINDOW)
        })
    }

    /// Count `tokens` sent by `user` on `model` at `at`.
    #[cfg(test)]
    fn sent_at(&self, user: UserId, model: Model, at: Instant, tokens: u64) {
        let mut sent = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        sent.entry((user, model))
            .or_default()
            .push_back((at, tokens));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPUS: Model = Model::ClaudeOpus55;

    fn user(id: i64) -> UserId {
        UserId::from_row(id)
    }

    #[test]
    fn a_member_past_their_budget_waits_while_another_goes_out() {
        let pace = Pace::default();
        let start = Instant::now();
        let (ana, bia) = (user(1), user(2));
        let half = budget(OPUS) / 2;
        pace.sent_at(ana, OPUS, start, half);
        assert_eq!(pace.free_at(ana, OPUS, start), None, "under it");
        let later = start + Duration::from_secs(20);
        pace.sent_at(ana, OPUS, later, half);
        let now = start + Duration::from_secs(30);
        assert_eq!(
            pace.free_at(ana, OPUS, now),
            Some(start + WINDOW),
            "until the first call is a minute old"
        );
        assert_eq!(pace.free_at(bia, OPUS, now), None, "bia is not ana");
        assert_eq!(
            pace.free_at(ana, Model::ClaudeSonnet55, now),
            None,
            "each model is its own limit"
        );
        assert_eq!(pace.free_at(ana, OPUS, start + WINDOW), None);
    }

    #[test]
    fn a_wait_lasts_until_enough_has_aged_out() {
        let pace = Pace::default();
        let start = Instant::now();
        let ana = user(1);
        let budget = budget(OPUS);
        pace.sent_at(ana, OPUS, start, 1);
        pace.sent_at(ana, OPUS, start + Duration::from_secs(10), 1);
        pace.sent_at(ana, OPUS, start + Duration::from_secs(20), budget);
        // The two small calls aging out leave the large one: still past it.
        assert_eq!(
            pace.free_at(ana, OPUS, start + Duration::from_secs(30)),
            Some(start + Duration::from_secs(20) + WINDOW)
        );
        assert_eq!(
            pace.free_at(ana, OPUS, start + Duration::from_secs(20) + WINDOW),
            None
        );
        assert!(
            pace.0.lock().unwrap().is_empty(),
            "what aged out is forgotten"
        );
    }

    #[test]
    fn only_what_was_not_read_from_the_cache_counts() {
        let pace = Pace::default();
        let ana = user(1);
        let usage = Usage {
            input: 10,
            output: 1_000_000,
            cache_write_5m: 20,
            cache_write_1h: 30,
            cache_read: 1_000_000,
        };
        pace.sent(ana, OPUS, &usage);
        let sent = pace.0.lock().unwrap();
        assert_eq!(sent[&(ana, OPUS)].iter().map(|(_, t)| t).sum::<u64>(), 60);
    }

    #[tokio::test]
    async fn a_stop_ends_the_wait_and_a_member_under_budget_never_waits() {
        let pace = Pace::default();
        let ana = user(1);
        assert!(pace.wait(ana, OPUS, || true).await, "nothing sent, no wait");
        pace.sent_at(ana, OPUS, Instant::now(), budget(OPUS));
        assert!(
            !pace.wait(ana, OPUS, || true).await,
            "stopped, not waited out"
        );
    }
}
