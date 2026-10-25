//! Withdrawing a purchase within its 7 days (CDC art. 49, the terms'
//! *Reembolso e direito de arrependimento*): the operator's
//! `scorsese-server credit withdraw`, since the money goes back through
//! Stripe's Dashboard by hand.
//!
//! **What is taken back is what is left of the purchase, its bonus
//! included** (#1537): the purchase's credits and bonus, or the paid balance
//! when that is less, since credits are one pool and what was spent was
//! spent. The bonus counts as spent in the same share as the rest, so a
//! purchase left whole gives back all its bonus, and one half spent half of
//! it. The reais refunded are the same share of what was paid, floored.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::reais;
use crate::ServerError;
use crate::accounts::users;
use crate::credits::{CreditError, dollars, history, ledger};
use crate::db::{self, Tx};

/// The withdrawal's own window, in days from the day the purchase was paid
/// to the day the customer asked, both in Brasília.
pub const WITHIN_DAYS: i32 = 7;

/// What withdrawing a purchase did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Withdrawn {
    /// What was taken back from the balance, in micro-dollars: 0 when
    /// nothing was left, and then nothing was written.
    pub taken_micros: i64,
    /// The part of it that was the purchase's bonus.
    pub bonus_micros: i64,
    /// The reais to refund through Stripe, in centavos.
    pub refund_centavos: i64,
    /// What the purchase cost, in centavos.
    pub paid_centavos: i64,
}

/// Withdraw the caller's purchase by Checkout `session`: take back what is
/// left of it, bonus included, as one `withdrawal` entry, and say what to
/// refund. `requested` is the day the customer asked, `YYYY-MM-DD` in
/// Brasília (today when `None`): the window is theirs, counted in days from
/// the day they paid, however late the operator gets to it.
///
/// Refused for a purchase not paid, asked for more than [`WITHIN_DAYS`] days
/// after it was paid, withdrawn already — and while a generation of theirs is
/// still waiting on its provider, since what it holds may yet come back and
/// is not spent. `tx` is scoped as the purchase's owner.
pub async fn withdraw(
    tx: &mut Tx,
    session: &str,
    requested: Option<&str>,
) -> Result<Withdrawn, CreditError> {
    let balance = ledger::locked_balance(tx).await?;
    let row: Option<(i64, String, i64, i64, i64, bool, bool)> = sqlx::query_as(
        "WITH asked AS (
             SELECT COALESCE($3::date, (now() AT TIME ZONE 'America/Sao_Paulo')::date) AS day
         )
         SELECT id, state, brl_centavos, credit_micros, bonus_micros,
                COALESCE((paid_at AT TIME ZONE 'America/Sao_Paulo')::date
                             BETWEEN asked.day - $2 AND asked.day
                         AND asked.day <= (now() AT TIME ZONE 'America/Sao_Paulo')::date,
                         false),
                EXISTS (SELECT 1 FROM credit_entries
                        WHERE payment_id = payments.id AND kind = 'withdrawal')
         FROM payments, asked WHERE stripe_session = $1 FOR UPDATE OF payments",
    )
    .bind(session)
    .bind(WITHIN_DAYS)
    .bind(requested)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((id, state, centavos, credit, bonus, within, withdrawn)) = row else {
        return Err(CreditError::Invalid(format!(
            "they have no purchase by session {session}"
        )));
    };
    if state != "paid" {
        return Err(CreditError::Invalid(format!(
            "purchase {session} is {state}, not paid: there is nothing to withdraw"
        )));
    }
    if withdrawn {
        return Err(CreditError::Invalid(format!(
            "purchase {session} was withdrawn already"
        )));
    }
    if !within {
        return Err(CreditError::Invalid(format!(
            "purchase {session} was not paid within the {WITHIN_DAYS} days before the day the \
             withdrawal was asked for"
        )));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM credit_entries r
             WHERE r.kind = 'reservation'
               AND NOT EXISTS (SELECT 1 FROM credit_entries s
                               WHERE s.settles = r.id AND s.kind = 'release'))",
    )
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Err(CreditError::Invalid(
            "a generation of theirs is still waiting on its provider; withdraw once it has \
             finished, so what it holds is counted right"
                .into(),
        ));
    }
    let withdrawn = share(centavos, credit, bonus, balance);
    if withdrawn.taken_micros > 0 {
        let detail = json!({
            "session": session,
            "bonus_micros": withdrawn.bonus_micros,
            "refund_centavos": withdrawn.refund_centavos,
        });
        ledger::withdraw(tx, withdrawn.taken_micros, id, detail).await?;
    }
    Ok(withdrawn)
}

/// `scorsese-server credit withdraw`: withdraw `email`'s purchase by
/// `session`, asked for on `requested`, and say what was taken back and what
/// to refund.
pub async fn run(
    pool: &PgPool,
    email: &str,
    session: &str,
    requested: Option<&str>,
) -> Result<String, ServerError> {
    if let Some(day) = requested.filter(|day| !history::is_day(day)) {
        return Err(
            CreditError::Invalid(format!("{day:?} is not a day; write it as YYYY-MM-DD")).into(),
        );
    }
    let user = users::find(pool, email).await?;
    let mut tx = db::scoped(pool, user)
        .await
        .map_err(ServerError::Database)?;
    let withdrawn = withdraw(&mut tx, session, requested).await?;
    let balance = ledger::balance(&mut tx)
        .await
        .map_err(ServerError::Database)?;
    tx.commit().await.map_err(ServerError::Database)?;
    Ok(if withdrawn.taken_micros == 0 {
        format!(
            "nothing is left of {email}'s purchase {session}; nothing taken back, nothing to refund"
        )
    } else {
        format!(
            "took back {} of {email}'s purchase {session} ({} of it bonus); balance now \
             {}\nrefund R$ {} of the R$ {} paid, in Stripe's Dashboard",
            dollars(withdrawn.taken_micros),
            dollars(withdrawn.bonus_micros),
            dollars(balance),
            reais(withdrawn.refund_centavos),
            reais(withdrawn.paid_centavos)
        )
    })
}

/// What withdrawing a purchase of `centavos` that credited `credit` and
/// `bonus` takes back from a paid `balance`.
fn share(centavos: i64, credit: i64, bonus: i64, balance: i64) -> Withdrawn {
    let credited = credit.saturating_add(bonus);
    let taken = balance.clamp(0, credited);
    let part = |of: i64| {
        let portion = i128::from(of) * i128::from(taken) / i128::from(credited.max(1));
        i64::try_from(portion).unwrap_or(0)
    };
    Withdrawn {
        taken_micros: taken,
        bonus_micros: part(bonus),
        refund_centavos: part(centavos),
        paid_centavos: centavos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_purchase_left_whole_gives_back_all_its_bonus_and_all_its_reais() {
        let whole = share(8_000, 16_000_000, 1_600_000, 20_000_000);
        assert_eq!(whole.taken_micros, 17_600_000);
        assert_eq!(whole.bonus_micros, 1_600_000);
        assert_eq!(whole.refund_centavos, 8_000);
    }

    #[test]
    fn a_purchase_half_spent_gives_back_half_its_bonus_and_half_its_reais() {
        let half = share(8_000, 16_000_000, 1_600_000, 8_800_000);
        assert_eq!(half.taken_micros, 8_800_000);
        assert_eq!(half.bonus_micros, 800_000);
        assert_eq!(half.refund_centavos, 4_000);
    }

    #[test]
    fn nothing_left_takes_nothing() {
        for balance in [0, -500] {
            let spent = share(8_000, 16_000_000, 1_600_000, balance);
            assert_eq!(spent.taken_micros, 0);
            assert_eq!(spent.bonus_micros, 0);
            assert_eq!(spent.refund_centavos, 0);
        }
    }

    #[test]
    fn a_purchase_with_no_bonus_refunds_its_share() {
        let quarter = share(2_000, 4_000_000, 0, 1_000_000);
        assert_eq!(quarter.taken_micros, 1_000_000);
        assert_eq!(quarter.bonus_micros, 0);
        assert_eq!(quarter.refund_centavos, 500);
    }
}
