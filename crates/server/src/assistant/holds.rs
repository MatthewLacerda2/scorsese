//! What one user's turns may take at once: how many run, and what their
//! calls in flight hold against the balance.
//!
//! A call to the model is charged after it answers, never reserved for (the
//! module doc of `credits`), so a turn decides before each call only that the
//! balance is above zero. One turn alone may then end a little below it, by
//! the one call that crossed. Several at once would each see the same last
//! dollar and each cross it. So **a call in flight holds what it may cost**
//! ([`estimate`]) until it is charged, and a turn — starting, or about to
//! call — counts the balance less what the user's *other* turns hold. A turn
//! alone is decided exactly as before; turns at once share the balance
//! rather than each spending it.
//!
//! Holds are in memory, kept by [`Holds`]: there is one server, every turn
//! runs in it, and a restart ends every running turn (`store::recover`) along
//! with its hold. Each decision is made under the user's lock
//! (`credits::ledger::lock`), the one their spending is decided under, so two
//! turns cannot both read the balance before either has placed its hold.
//!
//! **How many run at once** is bounded too ([`super::Assistant::at_once`]):
//! counted from `chat_turns` under the same lock, every project and
//! conversation together, when a turn starts or resumes from a pause.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_providers::chat::{Model, Request};
use scorsese_providers::prices::Checked;
use scorsese_providers::prices::chat::{self, Usage};

use super::AssistantError;
use crate::credits::{Product, ledger, price};
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// Output tokens a call is held for: a few steps of thinking and a tool
/// call, about what an editing call writes. Input is held at its full rate,
/// uncached, which a cached call pays a tenth of — so together the hold errs
/// high.
const OUTPUT_HELD: u64 = 4_096;

/// Bytes of a request taken for one token: four, the estimate `tool_calls`
/// already records replies by (migration `0020`).
const BYTES_PER_TOKEN: usize = 4;

/// What each running turn's call in flight holds, by turn. Cheap to clone;
/// clones share it.
#[derive(Clone, Default)]
pub(super) struct Holds(Arc<Mutex<HashMap<i64, (UserId, i64)>>>);

/// A call's hold, let go when dropped — once its charge is committed, or
/// however the call ended.
pub(super) struct Held {
    holds: Holds,
    turn: i64,
}

impl Holds {
    /// What `user`'s turns hold, but for `except`'s.
    fn of(&self, user: UserId, except: Option<i64>) -> i64 {
        let holds = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        holds
            .iter()
            .filter(|(turn, (owner, _))| *owner == user && Some(**turn) != except)
            .map(|(_, (_, micros))| *micros)
            .fold(0, i64::saturating_add)
    }

    /// Hold `micros` for `user`'s turn `turn`, in place of what it held.
    fn put(&self, user: UserId, turn: i64, micros: i64) -> Held {
        let mut holds = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        holds.insert(turn, (user, micros));
        Held {
            holds: self.clone(),
            turn,
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut holds = self.holds.0.lock().unwrap_or_else(PoisonError::into_inner);
        holds.remove(&self.turn);
    }
}

/// Why a call may not be made now: what the user can spend, and whether it
/// is their other turns' holds that leave nothing of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Short {
    /// The balance itself is spent.
    Spent(i64),
    /// Something is left, and the user's other turns' calls may spend it.
    Held(i64),
}

impl Short {
    /// What the person is told when a turn cannot start for it.
    pub(super) fn refusal(self) -> AssistantError {
        match self {
            Self::Spent(balance) => AssistantError::NoCredit(crate::credits::dollars(balance)),
            Self::Held(balance) => AssistantError::Held(crate::credits::dollars(balance)),
        }
    }
}

/// Take the user's lock in `tx` and say whether their turn `turn` — `None`
/// for one not begun — may call the model: what they can spend, less what
/// their other turns' calls in flight hold, must be above zero. The lock is
/// held until `tx` ends.
async fn decide(
    tx: &mut Tx,
    holds: &Holds,
    user: UserId,
    turn: Option<i64>,
) -> Result<Result<(), Short>, sqlx::Error> {
    ledger::lock(tx).await?;
    let spendable = ledger::spendable(tx).await?;
    if spendable <= 0 {
        return Ok(Err(Short::Spent(spendable)));
    }
    if spendable.saturating_sub(holds.of(user, turn)) <= 0 {
        return Ok(Err(Short::Held(spendable)));
    }
    Ok(Ok(()))
}

/// Admit a new turn, or a paused one resuming, for the user `tx` is scoped
/// to: refused when they already have as many running as they may, and —
/// unless `resuming`, whose first call decides for itself — when their
/// balance, less their running turns' holds, is spent. The lock is held
/// until `tx` ends, so the turn's row is written under it.
pub(super) async fn admit(
    state: &AppState,
    tx: &mut Tx,
    user: UserId,
    resuming: bool,
) -> Result<(), AssistantError> {
    let money = decide(tx, &state.assistant.holds, user, None).await?;
    let running: i64 =
        sqlx::query_scalar("SELECT count(*) FROM chat_turns WHERE state = 'running'")
            .fetch_one(&mut **tx)
            .await?;
    let most = state.assistant.at_once;
    if usize::try_from(running).unwrap_or(usize::MAX) >= most {
        return Err(AssistantError::AtOnce(most));
    }
    match money {
        Err(short) if !resuming => Err(short.refusal()),
        _ => Ok(()),
    }
}

/// Before `turn`'s next call, `request`: decide it may be made, and hold
/// what it may cost against the user's balance until the [`Held`] is
/// dropped. `spent` is what the turn has cost so far.
pub(super) async fn hold(
    state: &AppState,
    user: UserId,
    (turn, spent): (i64, i64),
    request: &Request,
) -> Result<Result<Held, Short>, sqlx::Error> {
    let holds = &state.assistant.holds;
    let mut tx = db::scoped(&state.pool, user).await?;
    if let Err(short) = decide(&mut tx, holds, user, Some(turn)).await? {
        return Ok(Err(short));
    }
    let left = state.assistant.cap_micros(request.model) - spent;
    let held = holds.put(user, turn, estimate(request).min(left.max(0)));
    tx.commit().await?;
    Ok(Ok(held))
}

/// What `request` may cost the user: its input at the model's full rate, as
/// if nothing were cached, and [`OUTPUT_HELD`] tokens out, times the
/// assistant's multiplier. Nothing for a model with no price, which is never
/// charged either.
pub(super) fn estimate(request: &Request) -> i64 {
    let bytes = request.system.len()
        + request
            .tools
            .iter()
            .map(|tool| tool.name.len() + tool.description.len() + tool.schema.to_string().len())
            .sum::<usize>()
        + request
            .messages
            .iter()
            .map(|message| message.get().len())
            .sum::<usize>();
    held(request.model, bytes / BYTES_PER_TOKEN)
}

/// What `input` tokens and [`OUTPUT_HELD`] out cost on `model` today, to the
/// user.
fn held(model: Model, input: usize) -> i64 {
    let Some(rate) = chat::rate(model, Checked::today()) else {
        return 0;
    };
    let usage = Usage {
        input: u64::try_from(input).unwrap_or(u64::MAX),
        output: OUTPUT_HELD,
        cache_write_5m: 0,
        cache_write_1h: 0,
        cache_read: 0,
    };
    price(Product::Assistant, usage.micros(rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: i64) -> UserId {
        UserId::from_row(id)
    }

    #[test]
    fn a_turn_counts_what_the_others_hold_and_never_its_own() {
        let holds = Holds::default();
        let first = holds.put(user(1), 10, 300);
        let _second = holds.put(user(1), 11, 200);
        let _theirs = holds.put(user(2), 12, 1_000);
        assert_eq!(holds.of(user(1), None), 500);
        assert_eq!(holds.of(user(1), Some(10)), 200);
        assert_eq!(holds.of(user(2), Some(10)), 1_000);
        drop(first);
        assert_eq!(holds.of(user(1), None), 200, "a dropped hold is let go");
    }

    #[test]
    fn a_hold_errs_high_and_grows_with_what_is_sent() {
        let model = Model::DEFAULT;
        let rate = chat::rate(model, Checked::today()).expect("the default model is priced");
        let nothing = held(model, 0);
        assert_eq!(
            nothing,
            price(Product::Assistant, OUTPUT_HELD * rate.output / 1_000_000)
        );
        assert!(held(model, 100_000) > nothing);
        let request = |text: &str| Request {
            model,
            system: text.to_owned(),
            tools: Vec::new(),
            messages: Vec::new(),
        };
        assert_eq!(estimate(&request("")), nothing);
        assert_eq!(
            estimate(&request(&"x".repeat(400_000))),
            held(model, 100_000)
        );
    }
}
