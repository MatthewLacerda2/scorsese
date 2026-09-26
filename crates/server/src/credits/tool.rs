//! `spending_history`: the history as a read-only tool, so a user on web MCP
//! (#539) can ask their own assistant "what did I spend this week?".
//!
//! **Described here, registered by #539.** Every other tool lives in
//! `scorsese-mcp`'s registry, whose `Tool` trait is synchronous, takes only
//! its arguments, and — by the rule in this crate's own `lib.rs` — never learns
//! about users or Postgres. This tool is nothing *but* a user's rows in
//! Postgres, and has no local equivalent: a `.scor` folder has no ledger. So it
//! cannot be one of that registry's tools as they stand, and the plumbing that
//! serves registry tools and server-side tools side by side, with the caller's
//! user attached, is exactly what web MCP builds. Until then this module is the
//! whole of the tool except the wire: its name, its self-description (every
//! argument described, `docs/mcp.md`'s rule, held by a test), how arguments
//! become a [`Filter`], and the text it answers with.

use serde_json::{Value, json};

use super::history::{Filter, History, KINDS};
use super::rates::reais;
use super::{CreditError, dollars};

/// How a client names it.
pub const NAME: &str = "spending_history";

/// What it does, in the words a client is shown. The first sentence stands
/// alone, as every tool's must.
pub const DESCRIPTION: &str = "List what the signed-in user has spent and paid in: every Veo shot, \
spoken line, assistant turn, monthly fee, top-up and refund, newest first, each with the balance \
after it. Read-only and free. A generation the provider failed shows as free; one that worked is \
charged whether or not it was kept. Amounts are in dollars — what the ledger is kept in — with an \
approximate figure in reais at the operator's dated display rate. Filter by project, kind or a \
range of days, and the answer includes what everything matched adds up to: ask with since and \
until to answer \"what did I spend this week?\".";

/// The JSON Schema of its arguments, every property described.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "project": {
                "type": "integer",
                "description": "Only rows about this project, by its id."
            },
            "kind": {
                "type": "string",
                "enum": KINDS,
                "description": "Only rows of this kind: a Veo shot, a spoken line, an assistant \
    turn, a top-up, a monthly fee or a refund."
            },
            "since": {
                "type": "string",
                "description": "Only rows on or after this day, in UTC, written YYYY-MM-DD."
            },
            "until": {
                "type": "string",
                "description": "Only rows on or before this day, in UTC, written YYYY-MM-DD."
            },
            "limit": {
                "type": "integer",
                "minimum": 1,
                "description": "At most this many rows, newest first. The total still covers \
    every row the filter matches."
            }
        }
    })
}

/// The filter a call's arguments ask for, or why they do not make one.
pub fn filter(arguments: &Value) -> Result<Filter, CreditError> {
    serde_json::from_value(arguments.clone())
        .map_err(|error| CreditError::Invalid(format!("the arguments do not read: {error}")))
}

/// The answer, as text an assistant can quote from.
pub fn answer(history: &History) -> String {
    let money = |micros: i64| match history.rate {
        Some(rate) => format!("{} (≈ {})", dollars(micros), reais(rate.centavos(micros))),
        None => dollars(micros),
    };
    let mut text = format!("Balance: {}.\n", money(history.balance_micros));
    if history.rate.is_none() {
        text.push_str("No display rate is set yet, so amounts are in dollars only.\n");
    }
    text.push_str(&format!(
        "{} matching rows, adding up to {}.\n",
        history.matched,
        money(history.total_micros)
    ));
    for row in &history.rows {
        let project = match (&row.project_name, row.project_id) {
            (Some(name), _) => format!(" — {name}"),
            (None, Some(id)) => format!(" — project {id}, since deleted"),
            (None, None) => String::new(),
        };
        let status = match row.status.as_str() {
            "free" => "free: the provider failed".to_owned(),
            "pending" => format!("{} held while it runs", money(-row.amount_micros)),
            _ => money(row.amount_micros),
        };
        text.push_str(&format!(
            "\n#{} on {} — {}{project}: {status}; balance after {}",
            row.id,
            row.when,
            row.memo,
            dollars(row.balance_after_micros)
        ));
    }
    text
}
