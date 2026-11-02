//! A paid generation the job queue gives up on after stops (#1645): its
//! reservation is settled with the give-up, since no handler runs for it
//! again.

use super::CreditError;
use super::generations::{Answer, finish, for_job};
use crate::db::Tx;

/// Settle what job `job` still holds once the queue has given up on it after
/// stops (`jobs::store::recover`), since no handler runs for it again:
/// charged when it was `sent` and the library holds what its `brief` made, so
/// output kept is never free; released otherwise. Nothing for a job nothing
/// pays for, or one already settled. The library item charged for, if any.
pub async fn settle(
    tx: &mut Tx,
    job: i64,
    sent: bool,
    brief: Option<&str>,
) -> Result<Option<i64>, CreditError> {
    let Some(paid) = for_job(tx, job).await? else {
        return Ok(None);
    };
    let kept: Option<i64> = match brief {
        Some(brief) if sent => {
            sqlx::query_scalar("SELECT id FROM library_items WHERE brief_hash = $1")
                .bind(brief)
                .fetch_optional(&mut **tx)
                .await?
        }
        _ => None,
    };
    let answer = match kept {
        Some(item) => Answer::Worked(Some(item)),
        None => Answer::Failed("the server stopped during it too often to try again".into()),
    };
    finish(tx, paid, &answer).await?;
    Ok(kept)
}
