//! An assistant call's charge: written to the ledger once the response has
//! said what it used, at its own model's rates (#705). Re-exported by
//! [`ledger`](super::ledger), where every other entry is written.

use scorsese_providers::chat::Model;
use scorsese_providers::prices::Checked;
use scorsese_providers::prices::chat::{self, Usage};
use serde_json::json;

use super::ledger::lock;
use super::{CreditError, Product, allowance, price};
use crate::db::Tx;

/// An assistant call, charged from the tokens its response reported.
#[derive(Debug, Clone)]
pub struct AssistantCall<'a> {
    /// The model it ran on, as the API names it.
    pub model: &'a str,
    /// The response's `usage`.
    pub usage: Usage,
    /// The project the turn was about.
    pub project: Option<i64>,
    /// What the user asked, in their words — what the history shows.
    pub prompt: &'a str,
    /// The chat turn the call was made in (#540), which folds a turn's calls
    /// into one row of the history.
    pub turn: Option<i64>,
}

/// What charging an assistant call wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charged {
    /// The entry.
    pub entry: i64,
    /// What the call cost at its model's rates, in micro-dollars.
    pub cost: i64,
    /// What the user was charged for it: the cost times its multiplier.
    pub micros: i64,
}

/// Charge one assistant call: its exact cost at its own model's rates
/// (#705), times [`Product::Assistant`]'s multiplier, the free allowance
/// paying first. Not reserved for, and not refused — the tokens are already
/// spent (see the module doc of [`credits`](super)).
pub async fn charge_assistant(
    tx: &mut Tx,
    call: &AssistantCall<'_>,
) -> Result<Charged, CreditError> {
    let rate = Model::from_id(call.model)
        .and_then(|model| chat::rate(model, Checked::today()))
        .ok_or_else(|| CreditError::Unpriced(call.model.into()))?;
    let cost = call.usage.micros(rate);
    let charged = price(Product::Assistant, cost);
    let detail = json!({
        "model": call.model,
        "usage": call.usage,
        "cost_micros": cost,
        "prompt": call.prompt,
    });
    lock(tx).await?;
    let (own, everyones) = allowance::own_and_everyones(tx).await?;
    let free = own.min(everyones).min(charged);
    let entry = sqlx::query_scalar(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, project_id, memo, detail,
                                     chat_turn_id)
         VALUES (member_id(), 'charge', $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(-charged)
    .bind(call.project)
    .bind(format!("Assistant: {}", call.model))
    .bind(detail)
    .bind(call.turn)
    .fetch_one(&mut **tx)
    .await?;
    allowance::grant(tx, entry, free).await?;
    Ok(Charged {
        entry,
        cost: i64::try_from(cost).unwrap_or(i64::MAX),
        micros: charged,
    })
}
