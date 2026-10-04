//! `model_calls` (#707): one typed row per call the assistant made to a
//! model, the same shape whichever vendor answered — what the maintainer
//! queries to see how each model performs and what it costs.

use scorsese_providers::chat::Reply;

use super::turns::Charge;
use crate::credits::ledger::Charged;
use crate::db::Tx;

/// Record the call `reply` answered, in the transaction that charged it.
pub(super) async fn record(
    tx: &mut Tx,
    charge: &Charge<'_>,
    reply: &Reply,
    charged: Charged,
) -> Result<(), sqlx::Error> {
    let usage = reply.usage;
    // A turn's calls are made one after another, so counting them is their
    // order.
    sqlx::query(
        "INSERT INTO model_calls
             (user_id, turn_id, position, model, vendor, input_tokens, output_tokens,
              thinking_tokens, cache_write_5m_tokens, cache_write_1h_tokens, cache_read_tokens,
              latency_ms, stop_reason, cost_micros, credit_entry_id)
         VALUES (member_id(), $1, (SELECT count(*) + 1 FROM model_calls WHERE turn_id = $1)::int,
                 $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(charge.turn)
    .bind(charge.model.id())
    .bind(charge.model.vendor().id())
    .bind(count(usage.input))
    .bind(count(usage.output))
    .bind(reply.thinking.map(count))
    .bind(count(usage.cache_write_5m))
    .bind(count(usage.cache_write_1h))
    .bind(count(usage.cache_read))
    .bind(i64::try_from(charge.latency.as_millis()).unwrap_or(i64::MAX))
    .bind(reply.stop.as_str())
    .bind(charged.cost)
    .bind(charged.entry)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// A token count as a `BIGINT`, saturating.
pub(super) fn count(tokens: u64) -> i64 {
    i64::try_from(tokens).unwrap_or(i64::MAX)
}
