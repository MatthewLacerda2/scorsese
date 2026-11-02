//! A dispute's debt (#1637): a paid balance a disputed purchase left below
//! zero, which stops every spend until a purchase brings it back
//! (`payments::reversed`).

use crate::db::Tx;

/// The paid balance, when a dispute has left it below zero (#1637): from
/// the newest dispute not won until a purchase (a top-up, its bonus, or a
/// refund the operator gave) brings the paid balance back to zero or above,
/// the account spends nothing, and the free allowance pays for nothing
/// either — else a disputed account would go on spending for free. A
/// balance below zero for any other reason (an assistant call that overran
/// it, after a debt was paid off) stops nothing the allowance can pay for.
/// A dispute that left the balance at zero or above owes nothing. The
/// balance is counted only at the dispute and at such entries: a
/// reservation released in between rises and falls again, and clears
/// nothing.
pub async fn owed(tx: &mut Tx) -> Result<Option<i64>, sqlx::Error> {
    let paid = super::ledger::balance(tx).await?;
    owed_at(tx, paid).await
}

/// [`owed`], for a paid balance read already.
pub(super) async fn owed_at(tx: &mut Tx, paid: i64) -> Result<Option<i64>, sqlx::Error> {
    if paid >= 0 {
        return Ok(None);
    }
    let disputed: bool = sqlx::query_scalar(
        "WITH debt AS (
             SELECT max(d.id) AS since FROM credit_entries d
             WHERE d.kind = 'dispute'
               AND NOT EXISTS (SELECT 1 FROM credit_entries w
                               WHERE w.kind = 'dispute_won' AND w.payment_id = d.payment_id)
         ), running AS (
             SELECT id, kind, sum(amount_micros) OVER (ORDER BY id) AS balance
             FROM credit_entries
         )
         SELECT debt.since IS NOT NULL
                AND NOT EXISTS (SELECT 1 FROM running r
                                WHERE r.id >= debt.since
                                  AND (r.id = debt.since
                                       OR r.kind IN ('top_up', 'bonus', 'refund'))
                                  AND r.balance >= 0)
         FROM debt",
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(disputed.then_some(paid))
}
