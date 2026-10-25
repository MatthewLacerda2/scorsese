//! A quote, and a Checkout Session started at it.

use scorsese_providers::fx::Rate;
use scorsese_providers::payments::{Order, SESSION_ID};
use serde::Serialize;
use sqlx::postgres::PgPool;

use super::{
    Bonus, EXPIRES_AFTER_SECONDS, MAXIMUM_CENTAVOS, MINIMUM_CENTAVOS, PaymentError, Payments, now,
};
use crate::credits::{MICROS_PER_CENT, dollars};
use crate::db::{self, UserId};

/// What an amount of reais buys, and the rate that says so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Quote {
    /// The reais, in centavos.
    pub brl_centavos: i64,
    /// The credits they buy, in micro-dollars.
    pub credit_micros: i64,
    /// The bonus credit the purchase adds on top, in micro-dollars (#1537).
    pub bonus_micros: i64,
    /// That bonus as a percentage of the credits: 0, or its tier's.
    pub bonus_percent: i64,
    /// Reais per dollar, as its source sent it.
    pub brl_per_usd: String,
    /// `awesomeapi` or `bcb_ptax`.
    pub rate_source: &'static str,
    /// The least one purchase may be, in centavos.
    pub minimum_centavos: i64,
    /// The most one purchase may be, in centavos.
    pub maximum_centavos: i64,
}

impl Payments {
    /// What `centavos` buy at the rate kept. Refused outside the purchase
    /// limits, and when the rate kept is too old to sell at. A server that
    /// has read no rate yet reads one first.
    pub async fn quote(&self, centavos: i64) -> Result<Quote, PaymentError> {
        if !(MINIMUM_CENTAVOS..=MAXIMUM_CENTAVOS).contains(&centavos) {
            return Err(PaymentError::OutOfRange);
        }
        if self.rates.is_empty() {
            let rates = self.rates.clone();
            tokio::task::spawn_blocking(move || rates.refresh())
                .await
                .map_err(|_| PaymentError::Interrupted)?
                .map_err(PaymentError::NoRate)?;
        }
        let rate = self.rates.current().ok_or(PaymentError::StaleRate)?;
        quoted(centavos, &rate, &self.bonus)
    }
}

/// `centavos` at `rate`, with the bonus its tier adds.
fn quoted(centavos: i64, rate: &Rate, bonus: &Bonus) -> Result<Quote, PaymentError> {
    let credit_micros = rate
        .micros_for(centavos)
        .filter(|micros| *micros > 0)
        .ok_or(PaymentError::OutOfRange)?;
    Ok(Quote {
        brl_centavos: centavos,
        credit_micros,
        bonus_micros: bonus.micros(centavos, credit_micros),
        bonus_percent: bonus.percent(centavos),
        brl_per_usd: rate.brl_per_usd.clone(),
        rate_source: rate.source.name(),
        minimum_centavos: MINIMUM_CENTAVOS,
        maximum_centavos: MAXIMUM_CENTAVOS,
    })
}

/// A session started: where to send the browser.
#[derive(Debug, Clone, Serialize)]
pub struct Begun {
    /// Stripe's hosted page.
    pub url: String,
    /// The session's id, which the return page brings back.
    pub session: String,
    /// What it was started at.
    pub quote: Quote,
}

/// Start a purchase of `centavos` for `user`, who was `shown` a rate: quote
/// it, open a Checkout Session for it at Stripe, and record the payment with
/// its rate and its bonus locked.
///
/// When there is no rate to sell at, or it is not the one `shown`, Stripe is
/// never called and nothing is written. A session Stripe made whose row then fails to be written is never
/// paid into from here — nobody is sent to it — and expires on its own.
pub async fn begin(
    pool: &PgPool,
    payments: &Payments,
    user: UserId,
    centavos: i64,
    shown: Option<&str>,
) -> Result<Begun, PaymentError> {
    let quote = payments.quote(centavos).await?;
    if shown.is_some_and(|shown| shown != quote.brl_per_usd) {
        return Err(PaymentError::RateMoved(Box::new(quote)));
    }
    let cent = i64::try_from(MICROS_PER_CENT).unwrap_or(1);
    let whole = |micros: i64| dollars(micros - micros % cent);
    let name = match quote.bonus_micros {
        0 => format!("scorsese: {} em créditos", whole(quote.credit_micros)),
        bonus => format!(
            "scorsese: {} em créditos + {} de bônus",
            whole(quote.credit_micros),
            whole(bonus)
        ),
    };
    let order = Order {
        centavos,
        name,
        success_url: format!("{}/credits/return?session={SESSION_ID}", payments.origin),
        cancel_url: format!("{}/spending", payments.origin),
        expires_at: now() + EXPIRES_AFTER_SECONDS,
        reference: user.get().to_string(),
    };
    let checkout = payments.checkout.clone();
    let session = tokio::task::spawn_blocking(move || checkout.create(&order))
        .await
        .map_err(|_| PaymentError::Interrupted)??;
    let url = session
        .url
        .clone()
        .ok_or_else(|| PaymentError::NoPage(session.id.clone()))?;

    let mut tx = db::scoped(pool, user).await?;
    sqlx::query(
        "INSERT INTO payments (user_id, stripe_session, brl_centavos, credit_micros,
                               bonus_micros, brl_per_usd, rate_source)
         VALUES (member_id(), $1, $2, $3, $4, $5, $6)",
    )
    .bind(&session.id)
    .bind(quote.brl_centavos)
    .bind(quote.credit_micros)
    .bind(quote.bonus_micros)
    .bind(&quote.brl_per_usd)
    .bind(quote.rate_source)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Begun {
        url,
        session: session.id,
        quote,
    })
}

#[cfg(test)]
mod tests {
    use scorsese_providers::fx::Source;

    use super::*;

    #[test]
    fn a_quote_floors_reais_into_credits_and_keeps_the_rate_as_sent() {
        let rate = Rate::parse("4.99", Source::Ptax).unwrap();
        let quote = quoted(5_000, &rate, &Bonus::none()).unwrap();
        assert_eq!(quote.credit_micros, 10_020_040);
        assert_eq!(quote.bonus_micros, 0);
        assert_eq!(quote.brl_per_usd, "4.99");
        assert_eq!(quote.rate_source, "bcb_ptax");
    }

    #[test]
    fn a_quote_carries_the_bonus_of_its_tier() {
        let rate = Rate::parse("4.99", Source::Ptax).unwrap();
        let quote = quoted(8_000, &rate, &Bonus::default()).unwrap();
        assert_eq!(quote.credit_micros, 16_032_064);
        assert_eq!(quote.bonus_micros, 1_603_206);
        assert_eq!(quote.bonus_percent, 10);
    }
}
