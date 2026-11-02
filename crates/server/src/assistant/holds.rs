//! What one user's turns may take at once: how many run, and what their
//! calls in flight hold against the balance.
//!
//! A call to the model is charged after it answers, never reserved for (the
//! module doc of `credits`), so a turn decides before each call what it can
//! pay for (#1653). **A call is made only when what is left pays for its
//! input and a short reply** ([`estimate`], `bound`), its input priced at the
//! most it can cost. A turn that cannot ends with the out-of-credit
//! sentence, before anything is sent. And **its reply is bounded by what is
//! left** ([`Held::most`]): the call asks for no more than what is left after
//! its input buys. A turn alone then ends below zero only by what its input
//! weighs past its estimate of four bytes a token, never by a reply it could
//! not pay for. The price of that bound is near the end of a balance: a
//! cached call costs far less than it is bounded for, so a reply may be cut
//! that the balance would in fact have paid for; the turn then ends with the
//! out-of-credit sentence ([`Short::Cut`]).
//!
//! Several turns at once would each see the same last dollar and each cross
//! it. So **a call in flight holds what it is likely to cost** ([`estimate`])
//! until it is charged, and a turn — starting, or about to call — counts the
//! balance less what the user's *other* turns hold. Turns at once share the
//! balance rather than each assuming all of it. The hold is an estimate, not
//! a bound — a call that thinks long writes more than it holds for — so what
//! turns at once can still cross by is what their calls cost past their
//! holds.
//!
//! Holds are in memory, kept by [`Holds`]: there is one server, every turn
//! runs in it, and a restart ends every running turn (`store::recover`) along
//! with its hold. Each decision is made under the user's lock
//! (`credits::ledger::lock`), the one their spending is decided under, so two
//! turns cannot both read the balance before either has placed its hold.
//!
//! **How many run at once** is bounded too ([`super::Assistant::at_once`]):
//! counted from `chat_turns` under the same lock, every project and
//! conversation together, when a turn starts or resumes from a pause. A turn
//! starts only when what is left pays for a short reply on its model, with
//! no input at all ([`least`]): one that could not make a single call is
//! refused before it is begun.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_providers::chat::{MAX_TOKENS, Model, Request};

use super::AssistantError;
use super::bound::{estimate, least, most_out};
use crate::credits::ledger;
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// What each running turn's call in flight holds, by turn. Cheap to clone;
/// clones share it.
#[derive(Clone, Default)]
pub(super) struct Holds(Arc<Mutex<HashMap<i64, (UserId, i64)>>>);

/// A call's hold, let go when dropped — once its charge is committed, or
/// however the call ended — and the longest reply it may ask for.
pub(super) struct Held {
    holds: Holds,
    turn: i64,
    /// The call's `max_tokens`: [`MAX_TOKENS`], or the reply what is left
    /// pays for after the call's input, when that is less.
    pub(super) most: u32,
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
            most: MAX_TOKENS,
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
    /// A reply was cut short at what was left, this much after it.
    Cut(i64),
}

impl Short {
    /// What the person is told when a turn cannot start for it.
    pub(super) fn refusal(self) -> AssistantError {
        match self {
            Self::Spent(balance) | Self::Cut(balance) => {
                AssistantError::NoCredit(crate::credits::dollars(balance))
            }
            Self::Held(balance) => AssistantError::Held(crate::credits::dollars(balance)),
        }
    }

    /// What a turn that may not make its next call ends saying.
    pub(super) fn ended(self) -> String {
        match self {
            Self::Spent(balance) => format!(
                "Stopped: your balance is {}. Add credit to carry on.",
                crate::credits::dollars(balance)
            ),
            Self::Cut(balance) => format!(
                "Stopped: your balance, {} now, did not pay for the rest of this reply. Add \
                 credit to carry on.",
                crate::credits::dollars(balance)
            ),
            Self::Held(balance) => format!(
                "Stopped: your balance is {}, and your other messages being worked on may spend \
                 it. Send this again once they finish, or add credit.",
                crate::credits::dollars(balance)
            ),
        }
    }
}

/// Take the user's lock in `tx` and say whether their turn `turn` — `None`
/// for one not begun — may make a call that costs at least `cost`: what
/// they can spend, less what their other turns' calls in flight hold, must
/// pay for it. What is left, if so. The lock is held until `tx` ends.
async fn decide(
    tx: &mut Tx,
    holds: &Holds,
    user: UserId,
    turn: Option<i64>,
    cost: i64,
) -> Result<Result<i64, Short>, sqlx::Error> {
    ledger::lock(tx).await?;
    let spendable = ledger::spendable(tx).await?;
    let cost = cost.max(1);
    if spendable < cost {
        return Ok(Err(Short::Spent(spendable)));
    }
    let left = spendable.saturating_sub(holds.of(user, turn));
    if left < cost {
        return Ok(Err(Short::Held(spendable)));
    }
    Ok(Ok(left))
}

/// Admit a new turn on `model`, or a paused one resuming, for the user `tx`
/// is scoped to: refused when they already have as many running as they
/// may, and — unless `resuming`, whose first call decides for itself — when
/// their balance, less their running turns' holds, cannot pay for the
/// least call ([`least`]). The lock is held until `tx` ends, so the turn's
/// row is written under it.
pub(super) async fn admit(
    state: &AppState,
    tx: &mut Tx,
    user: UserId,
    model: Model,
    resuming: bool,
) -> Result<(), AssistantError> {
    let money = decide(tx, &state.assistant.holds, user, None, least(model)).await?;
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

/// Before `turn`'s next call, `request`: decide it may be made — what is
/// left pays for its [`estimate`] — hold that against the user's balance
/// until the [`Held`] is dropped, and bound its reply by what is left.
/// `spent` is what the turn has cost so far.
pub(super) async fn hold(
    state: &AppState,
    user: UserId,
    (turn, spent): (i64, i64),
    request: &Request,
) -> Result<Result<Held, Short>, sqlx::Error> {
    let holds = &state.assistant.holds;
    let (input, estimate) = estimate(request);
    let mut tx = db::scoped(&state.pool, user).await?;
    let left = match decide(&mut tx, holds, user, Some(turn), estimate).await? {
        Ok(left) => left,
        Err(short) => return Ok(Err(short)),
    };
    let capped = state.assistant.cap_micros(request.model) - spent;
    let mut held = holds.put(user, turn, estimate.min(capped.max(0)));
    held.most = most_out(request.model, input, left);
    tx.commit().await?;
    Ok(Ok(held))
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
}
