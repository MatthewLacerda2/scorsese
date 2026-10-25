//! `scorsese-server credit free-usage` (#1530): what the free allowance paid,
//! per user per Brasília day, with the month to date and when each first paid.

use sqlx::postgres::PgPool;

use super::{CreditError, dollars, history};
use crate::ServerError;
use crate::db;

/// One day of a user's free usage, as `free_usage.sql` reads it.
#[derive(sqlx::FromRow)]
struct Day {
    email: String,
    day: String,
    free: i64,
    month_to_date: i64,
    first_paid: Option<String>,
}

/// `credit free-usage`: each user's line, then one per day they used the
/// allowance.
pub(super) async fn free_usage(
    pool: &PgPool,
    email: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<String, ServerError> {
    for day in [since, until].into_iter().flatten() {
        if !history::is_day(day) {
            return Err(CreditError::Invalid(format!(
                "{day:?} is not a day; write it as YYYY-MM-DD"
            ))
            .into());
        }
    }
    let mut tx = db::privileged(pool).await.map_err(ServerError::Database)?;
    let days: Vec<Day> = sqlx::query_as(include_str!("free_usage.sql"))
        .bind(email)
        .bind(since)
        .bind(until)
        .fetch_all(&mut *tx)
        .await
        .map_err(ServerError::Database)?;
    tx.commit().await.map_err(ServerError::Database)?;
    let mut lines = Vec::new();
    for (index, day) in days.iter().enumerate() {
        if index == 0 || days[index - 1].email != day.email {
            lines.push(match &day.first_paid {
                Some(first) => format!("{}, first paid {first}", day.email),
                None => format!("{}, never paid", day.email),
            });
        }
        lines.push(format!(
            "  {}  {} free, {} this month",
            day.day,
            dollars(day.free),
            dollars(day.month_to_date)
        ));
    }
    if lines.is_empty() {
        lines.push("nobody has used the free allowance in that range".to_owned());
    }
    Ok(lines.join("\n"))
}
