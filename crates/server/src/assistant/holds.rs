//! What one user's turns may take at once: how many run, and what their
//! calls in flight hold against the balance.
//!
//! A call to the model is charged after it answers, never reserved for (the
//! module doc of `credits`), so a turn decides before each call what it can
//! pay for (#1653). **A call is made only when what is left pays for its
//! input and a reply of [`OUTPUT_HELD`] tokens** ([`estimate`]); a turn that
//! cannot ends with the out-of-credit sentence, before anything is sent. And
//! **its reply is bounded by what is left** ([`Held::most`]): the call asks
//! for no more than [`MAX_TOKENS`], and no more than what is left after its
//! input buys. A turn alone then ends below zero only by what its input
//! weighs past its estimate, never by a reply it could not pay for.
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
//! starts only when what is left pays for a reply of [`OUTPUT_HELD`] tokens on
//! its model, with no input at all ([`least`]): one that could not make a
//! single call is refused before it is begun.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_providers::chat::{MAX_TOKENS, Model, Request};
use scorsese_providers::prices::Checked;
use scorsese_providers::prices::chat::{self, Rate, Usage};

use super::AssistantError;
use crate::credits::{Product, ledger, price};
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// Output tokens a call is held for: a few steps of thinking and a tool
/// call, about what an editing call writes, though one may write up to its
/// [`Held::most`]. Input is held at its full rate, uncached, where a cached
/// call pays a twentieth of it. It is also the shortest reply a call is
/// made for.
const OUTPUT_HELD: u64 = 4_096;

/// Bytes of a request's text taken for one token: four, the estimate
/// `tool_calls` already records replies by (migration `0020`).
const BYTES_PER_TOKEN: usize = 4;

/// Tokens a picture in the request is held for: Anthropic's largest
/// unscaled image, about 1.15 megapixels at 750 pixels a token — the figure
/// `tools::size` takes for a picture it cannot measure. Its base64 is never
/// counted as text, which would make one picture weigh tens of thousands.
const PICTURE_TOKENS: usize = 1_600;

/// What starts a picture's (or a document's) base64 in the wire's JSON: its
/// `data` key. A string's own quotes are escaped, so text that says this is
/// never mistaken for one.
const BASE64_KEY: &str = "\"data\":\"";

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
}

impl Short {
    /// What the person is told when a turn cannot start for it.
    pub(super) fn refusal(self) -> AssistantError {
        match self {
            Self::Spent(balance) => AssistantError::NoCredit(crate::credits::dollars(balance)),
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
            Self::Held(balance) => format!(
                "Stopped: your balance is {}, and your other messages being worked on may spend \
                 it. Send this again once they finish, or add credit.",
                crate::credits::dollars(balance)
            ),
        }
    }
}

/// Take the user's lock in `tx` and say whether their turn `turn` — `None`
/// for one not begun — may make a call that costs at least `least`: what
/// they can spend, less what their other turns' calls in flight hold, must
/// pay for it. What is left, if so. The lock is held until `tx` ends.
async fn decide(
    tx: &mut Tx,
    holds: &Holds,
    user: UserId,
    turn: Option<i64>,
    least: i64,
) -> Result<Result<i64, Short>, sqlx::Error> {
    ledger::lock(tx).await?;
    let spendable = ledger::spendable(tx).await?;
    let least = least.max(1);
    if spendable < least {
        return Ok(Err(Short::Spent(spendable)));
    }
    let left = spendable.saturating_sub(holds.of(user, turn));
    if left < least {
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

/// The least a call on `model` may cost: a reply of [`OUTPUT_HELD`] tokens,
/// with no input at all — what a turn must have left to be begun.
pub(super) fn least(model: Model) -> i64 {
    held(model, 0)
}

/// What `request` is likely to cost the user: its input at the model's full
/// rate, as if nothing were cached — text at [`BYTES_PER_TOKEN`], each
/// picture at [`PICTURE_TOKENS`] — and [`OUTPUT_HELD`] tokens out, times the
/// assistant's multiplier. Nothing for a model with no price, which is never
/// charged either. With the input tokens it counted.
fn estimate(request: &Request) -> (usize, i64) {
    let input = input_tokens(request);
    (input, held(request.model, input))
}

/// The tokens `request` sends, as [`estimate`] counts them.
fn input_tokens(request: &Request) -> usize {
    let tools: usize = request
        .tools
        .iter()
        .map(|tool| tool.name.len() + tool.description.len() + tool.schema.to_string().len())
        .sum();
    let (text, pictures) = request
        .messages
        .iter()
        .map(|message| weigh(message.get()))
        .fold(
            (request.system.len() + tools, 0),
            |(text, pictures), (t, p)| (text + t, pictures + p),
        );
    text / BYTES_PER_TOKEN + pictures * PICTURE_TOKENS
}

/// One message's text bytes and the pictures in it: everything but each
/// base64 string after [`BASE64_KEY`], which is one picture.
fn weigh(json: &str) -> (usize, usize) {
    let (mut text, mut pictures, mut rest) = (0, 0, json);
    while let Some(at) = rest.find(BASE64_KEY) {
        let start = at + BASE64_KEY.len();
        text += start;
        pictures += 1;
        // base64 has no quote or backslash, so the next quote ends it.
        rest = rest[start..]
            .find('"')
            .map_or("", |end| &rest[start + end..]);
    }
    (text + rest.len(), pictures)
}

/// What `input` tokens and [`OUTPUT_HELD`] out cost on `model` today, to the
/// user.
fn held(model: Model, input: usize) -> i64 {
    chat::rate(model, Checked::today()).map_or(0, |rate| priced(rate, input, OUTPUT_HELD))
}

/// What `input` tokens in, uncached, and `output` out cost at `rate`, to the
/// user.
fn priced(rate: Rate, input: usize, output: u64) -> i64 {
    let usage = Usage {
        input: u64::try_from(input).unwrap_or(u64::MAX),
        output,
        cache_write_5m: 0,
        cache_write_1h: 0,
        cache_read: 0,
    };
    price(Product::Assistant, usage.micros(rate))
}

/// The longest reply on `model`, after `input` tokens in, that `left` pays
/// for today: [`MAX_TOKENS`] when it pays for that, and never less than
/// [`OUTPUT_HELD`], which a call is not made without. Unbounded for a model
/// with no price, which is never charged.
fn most_out(model: Model, input: usize, left: i64) -> u32 {
    let Some(rate) = chat::rate(model, Checked::today()) else {
        return MAX_TOKENS;
    };
    let pays = |output: u64| priced(rate, input, output) <= left;
    let (mut paid, mut unpaid) = (OUTPUT_HELD, u64::from(MAX_TOKENS));
    if pays(unpaid) {
        return MAX_TOKENS;
    }
    while unpaid - paid > 1 {
        let middle = paid + (unpaid - paid) / 2;
        if pays(middle) {
            paid = middle;
        } else {
            unpaid = middle;
        }
    }
    u32::try_from(paid).unwrap_or(MAX_TOKENS)
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
            max_tokens: MAX_TOKENS,
        };
        assert_eq!(estimate(&request("")).1, nothing);
        assert_eq!(
            estimate(&request(&"x".repeat(400_000))).1,
            held(model, 100_000)
        );
    }

    #[test]
    fn a_picture_is_held_as_a_picture_and_never_as_its_base64() {
        let message = |json: String| {
            serde_json::value::RawValue::from_string(json).expect("the test's JSON holds")
        };
        let picture = "A".repeat(400_000);
        let with_picture = Request {
            model: Model::DEFAULT,
            system: String::new(),
            tools: Vec::new(),
            messages: vec![message(format!(
                "{{\"role\":\"user\",\"content\":[{{\"type\":\"image\",\"source\":\
                 {{\"type\":\"base64\",\"media_type\":\"image/jpeg\",\"data\":\"{picture}\"}}}}]}}"
            ))],
            max_tokens: MAX_TOKENS,
        };
        let around = with_picture.messages[0].get().len() - picture.len();
        assert_eq!(
            estimate(&with_picture).1,
            held(Model::DEFAULT, around / BYTES_PER_TOKEN + PICTURE_TOKENS)
        );
        // Words that merely say the key, escaped inside a string, are words.
        let words = "x".repeat(400);
        let quoting = Request {
            messages: vec![message(format!(
                "{{\"role\":\"user\",\"content\":\"\\\"data\\\":\\\"{words}\"}}"
            ))],
            ..with_picture
        };
        let length = quoting.messages[0].get().len();
        assert_eq!(
            estimate(&quoting).1,
            held(Model::DEFAULT, length / BYTES_PER_TOKEN)
        );
    }

    #[test]
    fn a_reply_is_as_long_as_what_is_left_pays_for() {
        let model = Model::DEFAULT;
        let rate = chat::rate(model, Checked::today()).expect("the default model is priced");
        let input = 20_000;
        let whole = priced(rate, input, u64::from(MAX_TOKENS));
        assert_eq!(most_out(model, input, whole), MAX_TOKENS);
        assert_eq!(most_out(model, input, i64::MAX), MAX_TOKENS);
        let short = most_out(model, input, whole / 2);
        assert!(
            short > u32::try_from(OUTPUT_HELD).unwrap() && short < MAX_TOKENS,
            "{short}"
        );
        assert!(priced(rate, input, u64::from(short)) <= whole / 2);
        assert!(
            priced(rate, input, u64::from(short) + 1) > whole / 2,
            "the longest it pays for"
        );
        // Never shorter than the reply a call is decided on.
        assert_eq!(u64::from(most_out(model, input, 0)), OUTPUT_HELD);
        assert_eq!(least(model), held(model, 0));
    }
}
